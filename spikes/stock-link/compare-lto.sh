#!/usr/bin/env bash
# Compares three LTO setups for the same guest and vendor code on every zkVM. Run `run-all.sh`
# first: it builds the vendor staticlibs, SDK directories and runners this script reuses.
#
#   split  vendor module full-LTO, guest ThinLTO: lld optimizes them in separate partitions, so
#          the guest cannot inline vendor functions
#   thin   vendor module ThinLTO (with summary): the guest can import and inline them
#   full   guest built by plain cargo with lto = "fat" against the SDK directory
#          (build-guest.sh): guest and vendor optimized together as one module
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
sizes=(${SIZES:-0 1000 10000})

# vendor | target spec | link args
vendors=(
    "openvm|riscv64ima-openvm-elf|-Ttext=0x00200800 --fatal-warnings"
    "zisk|riscv64ima-zisk-zkvm-elf|-T$here/linker/zisk.ld"
    "sp1|riscv64ima-succinct-zkvm-elf|-T$here/linker/sp1.ld"
)

for entry in "${vendors[@]}"; do
    IFS='|' read -r zkvm spec link <<<"$entry"
    lib="target/$spec/release/libzkvm_$zkvm.a"
    LTO=full ./vendor-archive.sh "$lib" "out/libzkvm_${zkvm}_full.a" >/dev/null
    LTO=thin ./vendor-archive.sh "$lib" "out/libzkvm_${zkvm}_thin.a" >/dev/null
    # shellcheck disable=SC2086
    {
        ./link-guest.sh "zkvm_${zkvm}_full" "out/guest-$zkvm-split.elf" $link >/dev/null 2>&1
        ./link-guest.sh "zkvm_${zkvm}_thin" "out/guest-$zkvm-thin.elf" $link >/dev/null 2>&1
        ./build-guest.sh "out/sdk/$zkvm" "out/guest-$zkvm-full.elf" >/dev/null 2>&1
    }
done

count() { "target/runner-$1/release/stock-link-runner" "$2" "$3" | awk '/^instructions/ {print $2}'; }

printf '%-8s %8s %10s %10s %10s %14s\n' zkvm input split thin full "full vs split"
for entry in "${vendors[@]}"; do
    zkvm=${entry%%|*}
    for n in "${sizes[@]}"; do
        split=$(count "$zkvm" "out/guest-$zkvm-split.elf" "$n")
        thin=$(count "$zkvm" "out/guest-$zkvm-thin.elf" "$n")
        full=$(count "$zkvm" "out/guest-$zkvm-full.elf" "$n")
        printf '%-8s %8s %10s %10s %10s %13.1f%%\n' "$zkvm" "$n" "$split" "$thin" "$full" \
            "$(echo "($full - $split) * 100 / $split" | bc -l)"
    done
done
echo
for entry in "${vendors[@]}"; do
    zkvm=${entry%%|*}
    for variant in split thin full; do
        elf="out/guest-$zkvm-$variant.elf"
        printf '%-20s %7s bytes  out-of-line zkvm_*: %s\n' "$zkvm-$variant" "$(wc -c <"$elf" | tr -d ' ')" \
            "$(/opt/homebrew/opt/llvm/bin/llvm-nm "$elf" | awk '$3 ~ /^zkvm_/ {printf "%s ", $3}')"
    done
done
