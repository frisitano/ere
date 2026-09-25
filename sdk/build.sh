#!/usr/bin/env bash
# Builds one zkVM SDK: a directory holding `libzkvm.a` (the vendor archive, one LLVM bitcode
# module exporting only the guest ABI in `abi.txt`) and `zkvm.ld` (the vendor linker script,
# which pulls the archive in with `INPUT(-lzkvm)`).
#
# Until zkVM teams publish SDKs, ere builds them from pinned vendor sources, with stock nightly
# Rust and `-Clinker-plugin-lto`. Each vendor crate is built for its own target spec in
# `targets/`, the generic target with only `os`/`vendor` changed, so the vendor's cfgs select its
# accelerated paths.
#
# With `software`, the SDK is the acceleration check's negative control: the vendor's runtime with
# every `zkvm_*` symbol replaced by the plain RISC-V implementations in `software/`.
#
# Usage: build.sh <openvm|zisk|sp1> <out-dir> [software]
# Requires: nightly Rust with rust-src, LLVM tools matching nightly's LLVM major (LLVM_BIN).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
zkvm=$1 software=${3:-}
mkdir -p "$2"
out=$(cd "$2" && pwd)
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm@22/bin}
export LLVM_BIN CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$here/target}"

case $zkvm in
    openvm) spec=riscv64ima-openvm-elf std=core,alloc,panic_abort ;;
    zisk) spec=riscv64ima-zisk-zkvm-elf std=core,alloc,panic_abort ;;
    sp1) spec=riscv64ima-succinct-zkvm-elf std=std,panic_abort ;;
    *) echo "unknown zkVM: $zkvm" >&2; exit 1 ;;
esac

build_staticlib() { # <crate dir> <target spec path> <build-std crates>
    (cd "$here/$1" && RUSTFLAGS='-Clinker-plugin-lto -Cpasses=lower-atomic --cfg getrandom_backend="custom"' \
        cargo +nightly build --release --locked -Zbuild-std="$3" \
        -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec \
        --target "$2")
}

build_staticlib "$zkvm" "$here/targets/$spec.json" "$std"
input='INPUT(-lzkvm)'
if [[ $software == software ]]; then
    # The generic guest target, as `ere-compiler-sdk` builds guests for it.
    build_staticlib software "$here/../crates/compiler/sdk/src/rust_rv64ima/riscv64ima-unknown-zkvm-elf.json" \
        std,panic_abort
    grep '^zkvm_' "$here/abi.txt" >"$out/accelerators.txt"
    grep -v '^zkvm_' "$here/abi.txt" >"$out/runtime.txt"
    "$here/vendor-archive.sh" "$CARGO_TARGET_DIR/riscv64ima-unknown-zkvm-elf/release/libzkvm_software.a" \
        "$out/libzkvm_software.a" "$out/accelerators.txt"
    "$here/vendor-archive.sh" "$CARGO_TARGET_DIR/$spec/release/libzkvm_$zkvm.a" "$out/libzkvm.a" \
        "$out/runtime.txt"
    rm "$out/accelerators.txt" "$out/runtime.txt"
    input='INPUT(-lzkvm -lzkvm_software)'
else
    "$here/vendor-archive.sh" "$CARGO_TARGET_DIR/$spec/release/libzkvm_$zkvm.a" "$out/libzkvm.a"
fi

{
    printf '/* Pulls in the SDK archive: a guest link needs only -T this script and -L its directory. */\n'
    printf '%s\n' "$input"
    # Native vendor members that must always be linked, see `vendor-archive.sh`.
    keep=$("$LLVM_BIN/llvm-nm" "$out"/libzkvm*.a 2>/dev/null |
        awk '$NF ~ /^__zkvm_sdk_keep_/ {print $NF}' | sort -u | paste -sd' ' -)
    [[ -n $keep ]] && printf 'EXTERN(%s)\n' "$keep"
    printf '\n'
    cat "$here/linker/$zkvm.ld"
} >"$out/zkvm.ld"
echo "$out"
