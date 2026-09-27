#!/usr/bin/env bash
# Links a guest object against a zkVM SDK, after checking the guest object against the ABI.
#
# The guest object is a static archive of LLVM bitcode (plus native compiler builtins) that defines
# `main`. It must define no symbol of the guest ABI in `abi.txt`, and leave nothing undefined that
# is not in it; otherwise the link could silently take a guest definition over the vendor's, or
# depend on one vendor's internals. If the SDK has a `zkvm.features` file, its ISA extensions are
# then added to the guest's code (below). The link is the one fixed command, so the ELF depends
# only on the guest object, the SDK and the linker.
#
# If the SDK carries an LLVM pass plugin (`zkvm-lto-plugin.so`, built for the linker's LLVM), the
# link loads it into the LTO pipeline. LLVM options come from two places, both passed as `-mllvm`:
# the SDK's `zkvm.llvm-args` (one per line, the vendor's options for every guest, e.g. SP1's
# scheduling direction), then the arguments after the output (this guest's tuning on this zkVM, e.g.
# `--inline-threshold=4749`), so a guest option can override a vendor one. Both are part of the
# link command, so the ELF depends on them too.
#
# `--software <pattern>[,<pattern>...]` links the SDK's plain RISC-V implementation
# (`libzkvm_software.a`) in place of the vendor's for every accelerator that matches a pattern (a
# symbol, or a prefix ending in `*`, e.g. `zkvm_u256_*`; `zkvm_*` for all), leaving the rest
# accelerated. The guest object and the SDK stay the same, so one link per configuration measures
# what each accelerator is worth. The vendor's own internal uses of an accelerator are unaffected.
#
# Usage: link.sh [--software <pattern>,...] <sdk-dir> <guest.a> <out.elf> [llvm-option...]
# Requires: `ld.lld` (LD_LLD) with an LLVM at least as new as the guest's and the SDK's bitcode, and
# `llvm-nm`, `llvm-ar`, `llvm-dis` and `llvm-as` (LLVM_BIN).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
software=
if [[ ${1:-} == --software ]]; then software=$2; shift 2; fi
sdk=$1 guest=$(cd "$(dirname "$2")" && pwd)/$(basename "$2") out=$3
shift 3
lto_args=()
if [[ -f $sdk/zkvm.llvm-args ]]; then
    while read -r arg; do [[ -n $arg ]] && lto_args+=(-mllvm "$arg"); done <"$sdk/zkvm.llvm-args"
fi
for arg in "$@"; do lto_args+=(-mllvm "$arg"); done
[[ -f $sdk/zkvm-lto-plugin.so ]] && lto_args+=("--load-pass-plugin=$sdk/zkvm-lto-plugin.so")
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm@22/bin}
LD_LLD=${LD_LLD:-ld.lld}

abi=$(sort -u "$here/abi.txt")
defined=$("$LLVM_BIN/llvm-nm" --defined-only --extern-only "$guest" 2>/dev/null |
    awk 'NF >= 3 && $(NF-1) ~ /^[TDBR]$/ {print $NF}' | sort -u)
undefined=$(comm -23 \
    <("$LLVM_BIN/llvm-nm" --undefined-only "$guest" 2>/dev/null | awk 'NF >= 2 {print $NF}' | sort -u) \
    <("$LLVM_BIN/llvm-nm" --defined-only "$guest" 2>/dev/null | awk 'NF >= 2 {print $NF}' | sort -u))

grep -qx main <<<"$defined" || { echo "$guest: does not define main" >&2; exit 1; }
if clash=$(comm -12 <(echo "$abi") <(echo "$defined")) && [[ -n $clash ]]; then
    echo "$guest: defines guest ABI symbols:" $clash >&2
    exit 1
fi
if extra=$(comm -13 <(echo "$abi") <(echo "$undefined")) && [[ -n $extra ]]; then
    echo "$guest: needs symbols outside the guest ABI:" $extra >&2
    exit 1
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/zkvm-link.XXXXXX")
trap 'rm -rf "$work"' EXIT

# The guest is built for plain RV64IM. The ISA extensions this zkVM supports (`zkvm.features`) are
# added to the guest's code here, so one guest object serves every zkVM. They go into each
# function's `target-features` attribute, because that attribute replaces, not extends, the
# features `ld.lld` would otherwise give code generation. Features are only ever added.
if [[ -s $sdk/zkvm.features ]]; then
    features=$(tr -d '[:space:]' <"$sdk/zkvm.features")
    [[ $features =~ ^(\+[a-z0-9.-]+)(,\+[a-z0-9.-]+)*$ ]] || {
        echo "$sdk/zkvm.features: expected +feature[,+feature...], got '$features'" >&2
        exit 1
    }
    members=()
    while IFS= read -r member; do members+=("$member"); done < <("$LLVM_BIN/llvm-ar" t "$guest")
    (cd "$work" && "$LLVM_BIN/llvm-ar" x "$guest")
    for member in "${members[@]}"; do
        [[ $(head -c 4 "$work/$member" | xxd -p) == 4243c0de ]] || continue
        "$LLVM_BIN/llvm-dis" "$work/$member" -o - |
            sed -E -e "s/\"target-features\"=\"\"/\"target-features\"=\"$features\"/g" \
                -e "s/(\"target-features\"=\"[^\"]+)\"/\1,$features\"/g" |
            "$LLVM_BIN/llvm-as" -o "$work/$member.tmp"
        mv "$work/$member.tmp" "$work/$member"
    done
    (cd "$work" && "$LLVM_BIN/llvm-ar" rcs guest.a "${members[@]}")
    guest=$work/guest.a
fi

# Accelerators replaced by the software archive: exported only from it, internal to the vendor's.
if [[ -n $software ]]; then
    [[ -f $sdk/libzkvm_software.a ]] || { echo "$sdk: no libzkvm_software.a for --software" >&2; exit 1; }
    regex=$(tr ',' '\n' <<<"$software" | sed -e 's/[.]/\\./g' -e 's/[*]$/.*/' -e 's/^/^/' -e 's/$/$/' |
        paste -sd'|' -)
    grep '^zkvm_' "$here/abi.txt" | grep -E "$regex" | sort -u >"$work/software.txt" || {
        echo "--software $software matches no accelerator in abi.txt" >&2
        exit 1
    }
    echo "software: $(paste -sd' ' - <"$work/software.txt")" >&2
    restrict() { # <archive> <out-dir> <file of symbols to keep public, or '-' for all but software.txt>
        mkdir -p "$2"
        local members=() member public
        while IFS= read -r member; do members+=("$member"); done < <("$LLVM_BIN/llvm-ar" t "$1")
        (cd "$2" && "$LLVM_BIN/llvm-ar" x "$1")
        for member in "${members[@]}"; do
            [[ $(head -c 4 "$2/$member" | xxd -p) == 4243c0de ]] || continue
            if [[ $3 == - ]]; then
                public=$("$LLVM_BIN/llvm-nm" --defined-only --extern-only "$2/$member" | awk '{print $NF}' |
                    grep -vxF -f "$work/software.txt" | paste -sd, -)
            else
                public=$(paste -sd, - <"$3")
            fi
            "$LLVM_BIN/opt" "$2/$member" -o "$2/$member.tmp" -passes='internalize,globaldce' \
                -internalize-public-api-list="$public"
            mv "$2/$member.tmp" "$2/$member"
        done
        (cd "$2" && "$LLVM_BIN/llvm-ar" rcs "../$(basename "$1")" "${members[@]}")
    }
    mkdir -p "$work/sdk"
    restrict "$(cd "$sdk" && pwd)/libzkvm.a" "$work/sdk/vendor" -
    restrict "$(cd "$sdk" && pwd)/libzkvm_software.a" "$work/sdk/software" "$work/software.txt"
    keep=$("$LLVM_BIN/llvm-nm" "$work/sdk/libzkvm_software.a" 2>/dev/null |
        awk '$NF ~ /^__zkvm_sdk_keep_/ {print $NF}' | sort -u | paste -sd' ' -)
    sed -e 's/^INPUT(-lzkvm)$/INPUT(-lzkvm -lzkvm_software)/' "$sdk/zkvm.ld" >"$work/sdk/zkvm.ld"
    [[ -n $keep ]] && printf 'EXTERN(%s)\n' "$keep" >>"$work/sdk/zkvm.ld"
    grep -q 'INPUT(-lzkvm -lzkvm_software)' "$work/sdk/zkvm.ld" || { echo "$sdk/zkvm.ld: no INPUT(-lzkvm)" >&2; exit 1; }
    sdk=$work/sdk
fi

"$LD_LLD" -T "$sdk/zkvm.ld" -L "$sdk" --gc-sections --lto-O3 ${lto_args[@]+"${lto_args[@]}"} -o "$out" "$guest"
echo "$out"
