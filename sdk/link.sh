#!/usr/bin/env bash
# Links a guest object against a zkVM SDK, after checking the guest object against the ABI.
#
# The guest object is a static archive of LLVM bitcode (plus native compiler builtins) that defines
# `main`. It must define no symbol of the guest ABI in `abi.txt`, and leave nothing undefined that
# is not in it; otherwise the link could silently take a guest definition over the vendor's, or
# depend on one vendor's internals. The link is the one fixed command, so the ELF depends only on
# the guest object, the SDK and the linker.
#
# Usage: link.sh <sdk-dir> <guest.a> <out.elf>
# Requires: `ld.lld` (LD_LLD) with an LLVM at least as new as the guest's and the SDK's bitcode, and
# `llvm-nm` (LLVM_BIN).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
sdk=$1 guest=$2 out=$3
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

"$LD_LLD" -T "$sdk/zkvm.ld" -L "$sdk" --gc-sections --lto-O3 -o "$out" "$guest"
echo "$out"
