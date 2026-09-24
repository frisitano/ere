#!/usr/bin/env bash
# Builds every vendor archive, links the zkVM-agnostic guest against each one (accelerated and
# software control), executes it on the zkVM's own executor and prints the instruction counts.
#
# Requirements: nightly Rust with rust-src, LLVM tools matching nightly's LLVM major
# (LLVM_BIN, default Homebrew), jq. ZisK's host emulator needs Homebrew gmp, libomp, libsodium.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
mkdir -p out

sizes=(${SIZES:-0 1000 10000})

# vendor | crate dir | target spec | build-std crates | link args
vendors=(
    "openvm|openvm|riscv64ima-openvm-elf|core,alloc,panic_abort|-Ttext=0x00200800 --fatal-warnings"
    "zisk|zisk|riscv64ima-zisk-zkvm-elf|core,alloc,panic_abort|-T$here/linker/zisk.ld"
    "sp1|sp1|riscv64ima-succinct-zkvm-elf|std,panic_abort|-T$here/linker/sp1.ld"
)

build_staticlib() { # <crate dir> <target spec> <build-std crates>
    (cd "$here/$1" && CARGO_TARGET_DIR="$here/target" \
        RUSTFLAGS='-Clinker-plugin-lto -Cpasses=lower-atomic --cfg getrandom_backend="custom"' \
        cargo +nightly build --release --quiet -Zbuild-std="$3" \
        -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec \
        --target "$here/targets/$2.json")
}

echo "== software control archive"
build_staticlib software riscv64ima-unknown-none-elf core,panic_abort
./vendor-archive.sh target/riscv64ima-unknown-none-elf/release/libzkvm_software.a \
    out/libzkvm_software.a software/exports.txt

for entry in "${vendors[@]}"; do
    IFS='|' read -r zkvm dir spec std link <<<"$entry"
    echo "== $zkvm"
    build_staticlib "$dir" "$spec" "$std"
    ./vendor-archive.sh "target/$spec/release/libzkvm_$zkvm.a" "out/libzkvm_$zkvm.a"
    ./vendor-archive.sh "target/$spec/release/libzkvm_$zkvm.a" "out/libzkvm_${zkvm}_nohash.a" \
        exports-without-hashes.txt
    # shellcheck disable=SC2086
    ./link-guest.sh "zkvm_$zkvm" "out/guest-$zkvm.elf" $link >/dev/null
    # shellcheck disable=SC2086
    ./link-guest.sh "zkvm_${zkvm}_nohash" "out/guest-$zkvm-software.elf" $link -lzkvm_software >/dev/null

    (cd runner && CARGO_TARGET_DIR="$here/target/runner-$zkvm" cargo build --release --quiet \
        --features "$zkvm")
done

echo
printf '%-8s %8s %14s %14s %8s\n' zkvm input accelerated software ratio
for entry in "${vendors[@]}"; do
    zkvm=${entry%%|*}
    runner="target/runner-$zkvm/release/stock-link-runner"
    for n in "${sizes[@]}"; do
        fast=$("$runner" "out/guest-$zkvm.elf" "$n" | awk '/^instructions/ {print $2}')
        slow=$("$runner" "out/guest-$zkvm-software.elf" "$n" | awk '/^instructions/ {print $2}')
        printf '%-8s %8s %14s %14s %7.1fx\n' "$zkvm" "$n" "$fast" "$slow" "$(echo "$slow / $fast" | bc -l)"
    done
done
