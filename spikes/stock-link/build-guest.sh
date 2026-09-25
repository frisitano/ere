#!/usr/bin/env bash
# Builds the guest with plain `cargo build` against one zkVM SDK directory, which holds exactly
# `libzkvm.a` (vendor archive) and `zkvm.ld` (vendor linker script, which pulls in the archive
# with `INPUT(-lzkvm)`). The guest's `.cargo/config.toml` and `lto = "fat"` do the rest.
#
# Usage: build-guest.sh <sdk-dir> <output-elf>
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
sdk=$(cd "$1" && pwd) output=$2
target=riscv64ima-unknown-none-elf
export CARGO_TARGET_DIR="$here/target/guest-sdk"

cd "$here/guest"
# Cargo does not track the SDK files, so relink every time.
cargo +nightly clean --release --quiet -p stock-link-guest
cargo +nightly build --release --quiet \
    --config "target.$target.rustflags = [\"-Clink-arg=-L$sdk\", \"-Clink-arg=-T$sdk/zkvm.ld\"]"
cp "$CARGO_TARGET_DIR/$target/release/stock-link-guest" "$here/$output"
echo "$output"
