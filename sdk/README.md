# zkVM SDKs

A guest built once, naming no zkVM, gets its runtime and accelerators at link time from a zkVM SDK.
An SDK is a directory with two files, as in the zkvm-standards "Static Library and Linker Script"
proposal:

```text
<sdk>/
├── libzkvm.a   one LLVM bitcode module exporting exactly the guest ABI (`abi.txt`)
└── zkvm.ld     the vendor linker script; `INPUT(-lzkvm)` pulls the archive into the link
```

Until zkVM teams publish SDKs, `build.sh` builds them from pinned vendor sources with stock nightly
Rust.

## Build an SDK

```bash
sdk/build.sh <openvm|zisk|sp1> <out-dir>            # the SDK
sdk/build.sh <openvm|zisk|sp1> <out-dir> software   # its negative control
```

Requirements: nightly Rust with `rust-src`, and LLVM tools no older than nightly's LLVM
(`LLVM_BIN`, default Homebrew `llvm@22`).

Each vendor crate (`openvm/`, `zisk/`, `sp1/`) is a `staticlib` built for its own target in
`targets/`: the generic target with only `os`/`vendor` (and, for OpenVM, `+unaligned-scalar-mem`)
changed, so the vendor's cfgs select its accelerated paths. `vendor-archive.sh` then merges the
archive's bitcode into one module and internalizes everything outside the ABI, so the vendor's
`core`, panic handler and allocator cannot collide with the guest's. A native vendor member that
rustc kept out of LTO (OpenVM's `memcpy`) gets an anchor that `zkvm.ld` names in `EXTERN`, so it is
always linked instead of losing to the guest's weak `compiler_builtins`.

The `software` control keeps the vendor's runtime and replaces every `zkvm_*` symbol with the plain
RISC-V implementations in `software/` (`revm-precompile`'s pure-Rust backends). A guest linked
against it computes the same results with no acceleration, which is the baseline for the
acceleration check.

## Build and link a guest

The guest is a `staticlib` defining `int main(void)`, built for the generic
`riscv64ima-unknown-zkvm-elf` target (`crates/compiler/sdk`) with `lto = "fat"` and
`-Clinker-plugin-lto`. It uses `ere-platform-zkvm` for `Platform`, its panic hook and `getrandom`.
`ere-compiler-sdk`'s `SdkRustRv64ima` does both steps; for a prebuilt guest object:

```bash
LD_LLD=ld.lld sdk/link.sh <sdk> <guest.a> <guest.elf>
```

`link.sh` rejects a guest object that does not define `main`, defines an ABI symbol, or needs a
symbol outside the ABI. It then runs the one fixed command,
`ld.lld -T <sdk>/zkvm.ld -L <sdk> --gc-sections --lto-O3 -o <guest.elf> <guest.a>`. The input order
is part of it: where both sides carry a weak copy of the same compiler builtin, the order decides
which copy is linked, and so the ELF bytes and the verification key.

## Vendor notes

- OpenVM ships no linker script; `linker/openvm.ld` follows `openvm-platform`'s memory layout and
  declares `PHDRS`, so the ELF headers are not loaded inside the stack below `0x0020_0800`.
- OpenVM's `openvm-mem` needs `+unaligned-scalar-mem`; built without it, it stops OpenVM's
  executor.
- SP1's `libzkevm` v6.6.0 does not follow the standard encoding for `zkvm_bls12_*` and
  `zkvm_ripemd160`. The SDK pins the fix ere's SP1 test guests use until
  succinctlabs/sp1#2865 is released.
- SP1's executor rejects misaligned loads, so the generic guest target does not enable
  `+unaligned-scalar-mem`, although the zkvm-standards RISC-V target requires `Zicclsm`.
- The SDKs still export a few symbols outside the ABI that vendor assembly references (`__start`
  on OpenVM and SP1, `ZISK_BUMP_HEAP_POS`/`ZISK_BUMP_HEAP_TOP` on ZisK).
