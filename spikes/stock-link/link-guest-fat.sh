#!/usr/bin/env bash
# Full-LTO build in the usual two steps, like `clang -flto -c` followed by `ld.lld`:
#
# 1. rustc compiles the guest with `-Clto=fat --emit=llvm-bc`: one bitcode module holding the
#    guest and `core`, with no ThinLTO summary.
# 2. rust-lld links that module with the vendor archive (`vendor-archive.sh`, LTO=full). Both are
#    plain bitcode modules, so lld's LTO optimizes guest and vendor together as one module.
#
# `compiler_builtins` is never bitcode, so its rlib is passed to the link as a native archive.
#
# Usage: link-guest-fat.sh <vendor-lib-name> <output-elf> [extra link args...]
#   e.g. link-guest-fat.sh zkvm_openvm out/guest-openvm.elf -Ttext=0x00200800
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
lib=$1 output=$2
shift 2

target=riscv64ima-unknown-none-elf
export CARGO_TARGET_DIR="$here/target/guest-fat"
RUSTFLAGS="-Cpasses=lower-atomic -Cembed-bitcode=yes" cargo +nightly rustc --release --quiet \
    --manifest-path "$here/guest/Cargo.toml" \
    -Zbuild-std=core,panic_abort -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec \
    --target "$here/targets/$target.json" -- -Clto=fat --emit=llvm-bc

deps="$CARGO_TARGET_DIR/$target/release/deps"
guest_bc=$(ls -t "$deps"/stock_link_guest-*.bc | head -1)
builtins=$(ls -t "$deps"/libcompiler_builtins-*.rlib | head -1)
lld=$(ls "$(rustc +nightly --print sysroot)"/lib/rustlib/*/bin/rust-lld)

"$lld" -flavor gnu -o "$output" "$guest_bc" "-L$here/out" "-l$lib" "$builtins" \
    --gc-sections -plugin-opt=O3 -plugin-opt=mcpu=generic-rv64 "$@"
echo "$output"
