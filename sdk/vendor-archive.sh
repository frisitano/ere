#!/usr/bin/env bash
# Turns a Rust staticlib built with `-Clinker-plugin-lto` into a vendor archive that exports only
# the zkVM guest ABI listed in `abi.txt`.
#
# All bitcode members are merged into one module and every other symbol is internalized, so the
# vendor's copy of `core`, its panic handler and its allocator cannot collide with the guest's.
# The result is still bitcode, so the guest link can inline across the `zkvm_*` boundary.
# Native (non-bitcode) members are kept as they are.
#
# Usage: [LTO=thin|full] vendor-archive.sh <input.a> <output.a> [exports-file]
#   LTO=thin writes ThinLTO bitcode with a summary, so a ThinLTO guest can import and
#   inline from it; LTO=full (default) writes a plain module for a full-LTO link.
#   The export list defaults to `abi.txt`, the zkVM guest ABI.
set -euo pipefail

LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm@22/bin}
lto=${LTO:-full}
here=$(cd "$(dirname "$0")" && pwd)
input=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
output=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
exports=$(cd "$(dirname "${3:-$here/abi.txt}")" && pwd)/$(basename "${3:-abi.txt}")

work=$(mktemp -d "${AGENT_TMPDIR:-${TMPDIR:-/tmp}}/vendor-archive.XXXXXX")
cd "$work"
"$LLVM_BIN/llvm-ar" x "$input"

bitcode=() native=()
for member in *; do
    # DROP is a glob of native members to leave out of the SDK, see `build.sh`.
    [[ -n ${DROP:-} && $member == $DROP ]] && continue
    if [[ $(head -c 4 "$member" | xxd -p) == 4243c0de ]]; then bitcode+=("$member"); else native+=("$member"); fi
done

# A native member other than `compiler_builtins` is vendor code rustc kept out of LTO, for example
# OpenVM's `#![no_builtins]` memcpy. The guest's weak `compiler_builtins` definitions would stop the
# linker from ever extracting it, so give it an anchor symbol that `zkvm.ld` names in `EXTERN`,
# which links it unconditionally and lets its strong definitions win.
anchor=0 archive=$(basename "$output" .a)
for member in "${native[@]}"; do
    [[ $member == compiler_builtins-* ]] && continue
    "$LLVM_BIN/llvm-objcopy" --add-symbol "__zkvm_sdk_keep_${archive}_$anchor=0,global" "$member"
    anchor=$((anchor + 1))
done

"$LLVM_BIN/llvm-link" "${bitcode[@]}" -o merged.bc

# Module-level asm (for example a `_start` in `global_asm!`) can name a Rust symbol the IR does not
# see as used. Keep every symbol the module both defines and references, or it would be removed.
"$LLVM_BIN/llvm-nm" merged.bc | awk '$1 == "U" {print $2}' | sort -u > undefined.txt
"$LLVM_BIN/llvm-nm" --defined-only merged.bc | awk '{print $NF}' | sort -u > defined.txt
public=$( (cat "$exports"; comm -12 undefined.txt defined.txt) | sort -u | paste -sd, -)

"$LLVM_BIN/opt" merged.bc -o vendor.bc \
    -passes='internalize,globaldce' $([[ $lto == thin ]] && echo --thinlto-bc) \
    -internalize-public-api-list="$public"

rm -f "$output"
"$LLVM_BIN/llvm-ar" rcs "$output" vendor.bc "${native[@]}"
echo "$output: 1 bitcode module, ${#native[@]} native members"
