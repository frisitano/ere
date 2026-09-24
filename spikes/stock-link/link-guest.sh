#!/usr/bin/env bash
# Builds the zkVM-agnostic guest with stock nightly Rust and links it against one vendor archive
# with linker-plugin LTO. Only the link line differs between zkVMs.
#
# GUEST_RUSTFLAGS adds rustc flags for the guest, e.g. to choose its LTO mode.
#
# Usage: link-guest.sh <vendor-lib-name> <output-elf> [extra link args...]
#   e.g. link-guest.sh zkvm_openvm out/guest-openvm.elf -Ttext=0x00200800
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
lib=$1 output=$2
shift 2

rustflags=(-Clinker-plugin-lto -Cpasses=lower-atomic ${GUEST_RUSTFLAGS:-} "-Clink-arg=-L$here/out" "-Clink-arg=-l$lib")
for arg in "$@"; do rustflags+=("-Clink-arg=$arg"); done

export CARGO_TARGET_DIR="$here/target/guest-$lib"
# Cargo does not track the vendor archive, so force the guest to relink against the current one.
cargo +nightly clean --release --quiet --manifest-path "$here/guest/Cargo.toml" -p stock-link-guest \
    --target "$here/targets/riscv64ima-unknown-none-elf.json" -Zjson-target-spec
RUSTFLAGS="${rustflags[*]}" cargo +nightly build --release \
    --manifest-path "$here/guest/Cargo.toml" \
    -Zbuild-std=core,panic_abort -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec \
    --target "$here/targets/riscv64ima-unknown-none-elf.json"
cp "$CARGO_TARGET_DIR/riscv64ima-unknown-none-elf/release/stock-link-guest" "$output"
echo "$output"
