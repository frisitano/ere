#!/usr/bin/env bash
# Builds one zkVM SDK: a directory holding `libzkvm.a` (the vendor archive, one LLVM bitcode
# module exporting only the guest ABI in `abi.txt`), `zkvm.ld` (the vendor linker script, which
# pulls the archive in with `INPUT(-lzkvm)`) and, if the zkVM supports more than RV64IM,
# `zkvm.features` (the LLVM target features `link.sh` adds to the guest's code), optionally
# `zkvm-lto-plugin.so` (an LLVM pass plugin `link.sh` runs at the end of LTO), and
# `libzkvm_software.a` (every `zkvm_*` accelerator in plain RISC-V, from `software/`, which
# `link.sh --software` links in place of the vendor's for the accelerators it names).
#
# Until zkVM teams publish SDKs, ere builds them from pinned vendor sources, with stock nightly
# Rust and `-Clinker-plugin-lto`. Each vendor crate is built for its own target spec in
# `targets/`, the generic target with only `os`/`vendor` changed, so the vendor's cfgs select its
# accelerated paths.
#
# Usage: build.sh <openvm|zisk|sp1> <out-dir>
# Requires: the pinned nightly (RUST_TOOLCHAIN) with rust-src, and LLVM tools no older than its
# LLVM (LLVM_BIN).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
zkvm=$1
mkdir -p "$2"
out=$(cd "$2" && pwd)
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm@22/bin}
# The pinned nightly (LLVM 22.1.0). Guest objects must come from an LLVM no newer than the
# linker's, so the SDK and the `build-guest-object` action use the same one.
RUST_TOOLCHAIN=${RUST_TOOLCHAIN:-nightly-2026-03-17}
export LLVM_BIN CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$here/target}"

# `llvm_args`: LLVM options the vendor's own build passes for every guest, which `link.sh` passes to
# the link (`zkvm.llvm-args`). Guest-specific tuning is not here but in the link's own arguments.
#
# `features`: LLVM target features the zkVM supports beyond RV64IM, which `link.sh` adds to the
# guest's code at link time (`zkvm.features`). They match what each vendor's own guest builds use:
# OpenVM's target enables misaligned scalar access; ZisK's carries Zba/Zbb/Zbkb/Zbs and proves
# misaligned access more cheaply than the byte loads it replaces. SP1's executor rejects misaligned
# loads and has no bitmanip, so it gets none.
case $zkvm in
    openvm) spec=riscv64ima-openvm-elf std=core,alloc,panic_abort features=+unaligned-scalar-mem ;;
    zisk) spec=riscv64ima-zisk-zkvm-elf std=core,alloc,panic_abort
        features=+zba,+zbb,+zbkb,+zbs,+unaligned-scalar-mem plugin=zisk-dma ;;
    sp1) spec=riscv64ima-succinct-zkvm-elf std=std,panic_abort features=
        # `cargo prove build` compiles every SP1 guest with these.
        llvm_args=(-misched-prera-direction=bottomup -misched-postra-direction=bottomup) ;;
    *) echo "unknown zkVM: $zkvm" >&2; exit 1 ;;
esac

build_staticlib() { # <crate dir> <target spec path> <build-std crates> [target features]
    (cd "$here/$1" && RUSTFLAGS="-Clinker-plugin-lto -Cpasses=lower-atomic --cfg getrandom_backend=\"custom\"${4:+ -Ctarget-feature=$4}" \
        cargo "+$RUST_TOOLCHAIN" build --release --locked -Zbuild-std="$3" \
        -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec \
        --target "$2")
}

# The vendor's own code gets the same features as the guest's, as in the vendor's own builds.
build_staticlib "$zkvm" "$here/targets/$spec.json" "$std" "$features"
input='INPUT(-lzkvm)'
"$here/vendor-archive.sh" "$CARGO_TARGET_DIR/$spec/release/libzkvm_$zkvm.a" "$out/libzkvm.a"
# The software fallback, for the generic guest target with this zkVM's features, like the guest.
build_staticlib software "$here/../crates/compiler/sdk/src/rust_rv64ima/riscv64ima-unknown-zkvm-elf.json" \
    std,panic_abort "$features"
grep '^zkvm_' "$here/abi.txt" >"$out/accelerators.txt"
"$here/vendor-archive.sh" "$CARGO_TARGET_DIR/riscv64ima-unknown-zkvm-elf/release/libzkvm_software.a" \
    "$out/libzkvm_software.a" "$out/accelerators.txt"
rm "$out/accelerators.txt"

{
    printf '/* Pulls in the SDK archive: a guest link needs only -T this script and -L its directory. */\n'
    printf '%s\n' "$input"
    # Native vendor members that must always be linked, see `vendor-archive.sh`.
    keep=$("$LLVM_BIN/llvm-nm" "$out/libzkvm.a" 2>/dev/null |
        awk '$NF ~ /^__zkvm_sdk_keep_/ {print $NF}' | sort -u | paste -sd' ' -)
    [[ -n $keep ]] && printf 'EXTERN(%s)\n' "$keep"
    printf '\n'
    cat "$here/linker/$zkvm.ld"
} >"$out/zkvm.ld"
if [[ -n $features ]]; then echo "$features" >"$out/zkvm.features"; else rm -f "$out/zkvm.features"; fi
if [[ -n ${llvm_args[*]:-} ]]; then printf '%s\n' "${llvm_args[@]}" >"$out/zkvm.llvm-args"; else rm -f "$out/zkvm.llvm-args"; fi

# The zkVM's LLVM pass plugin (`plugins/`), run by `link.sh` at the end of LTO. It is native code for
# this machine and must be built against the LLVM of the `ld.lld` that loads it.
if [[ -n ${plugin:-} ]]; then
    cmake -S "$here/plugins/$plugin" -B "$CARGO_TARGET_DIR/plugin-$plugin" -DCMAKE_BUILD_TYPE=Release \
        -DLLVM_DIR="$LLVM_BIN/../lib/cmake/llvm" -DCMAKE_C_COMPILER="$LLVM_BIN/clang" \
        -DCMAKE_CXX_COMPILER="$LLVM_BIN/clang++" >/dev/null
    cmake --build "$CARGO_TARGET_DIR/plugin-$plugin" >/dev/null
    cp "$CARGO_TARGET_DIR/plugin-$plugin/zkvm-lto-plugin.so" "$out/"
else
    rm -f "$out/zkvm-lto-plugin.so"
fi
echo "$out"
