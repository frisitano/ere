#!/usr/bin/env bash
# Turns a Rust staticlib built with `-Clinker-plugin-lto` into a vendor archive that exports only
# the standard symbols listed in `exports.txt`.
#
# All bitcode members are merged into one module and every other symbol is internalized, so the
# vendor's copy of `core`, its panic handler and its allocator cannot collide with the guest's.
# The result is still bitcode, so the guest link can inline across the `zkvm_*` boundary.
# Native (non-bitcode) members are kept as they are.
#
# Usage: vendor-archive.sh <input.a> <output.a> [exports-file]
#   The export list defaults to `exports.txt`.
set -euo pipefail

LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
here=$(cd "$(dirname "$0")" && pwd)
input=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
output=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
exports=$(cd "$(dirname "${3:-$here/exports.txt}")" && pwd)/$(basename "${3:-exports.txt}")

work=$(mktemp -d "${AGENT_TMPDIR:-${TMPDIR:-/tmp}}/vendor-archive.XXXXXX")
cd "$work"
"$LLVM_BIN/llvm-ar" x "$input"

bitcode=() native=()
for member in *; do
    if [[ $(head -c 4 "$member" | xxd -p) == 4243c0de ]]; then bitcode+=("$member"); else native+=("$member"); fi
done

"$LLVM_BIN/llvm-link" "${bitcode[@]}" -o merged.bc
"$LLVM_BIN/opt" merged.bc -o vendor.bc \
    -passes='internalize,globaldce' \
    -internalize-public-api-list="$(paste -sd, "$exports")"

rm -f "$output"
"$LLVM_BIN/llvm-ar" rcs "$output" vendor.bc "${native[@]}"
echo "$output: 1 bitcode module, ${#native[@]} native members"
