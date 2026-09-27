# zkVM SDKs

A guest built once, naming no zkVM, gets its runtime and accelerators at link time from a zkVM SDK.
An SDK is a directory with the two files of the zkvm-standards "Static Library and Linker Script"
proposal, plus an optional list of ISA extensions:

```text
<sdk>/
├── libzkvm.a       one LLVM bitcode module exporting exactly the guest ABI (`abi.txt`)
├── zkvm.ld         the vendor linker script; `INPUT(-lzkvm)` pulls the archive into the link
├── zkvm.features   optional: ISA extensions beyond RV64IM, added to the guest's code at link time
└── zkvm-lto-plugin.so  optional: LLVM pass plugin run at the end of LTO (native, per host)
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

## Published SDKs

Guest CI does not build SDKs. Pushing a tag `sdk-*` runs `.github/workflows/release-sdk.yml`, which
builds the three SDKs (in a clean Ubuntu 24.04 container with LLVM 22 only) and publishes, as
release assets, `sdk-<zkvm>.tar.gz`, `ere-link-tools.tar.gz` (`link.sh`, `abi.txt`, the generic
guest target spec and `rust-toolchain`, the pinned nightly) and `SHA256SUMS`. ere-guests' reusable
`link-guest` workflow and `build-guest-object` action take a release tag and use these assets, so
one release pins the vendor runtimes, the link tools and the guest toolchain together. The ZisK
plugin in the release is built for Linux x86_64 against LLVM 22.

## Build and link a guest

The guest is a `staticlib` defining `int main(void)`, built for the generic
`riscv64ima-unknown-zkvm-elf` target (`crates/compiler/sdk`) with `lto = "fat"` and
`-Clinker-plugin-lto`. It uses `ere-platform-zkvm` for `Platform`, its panic hook and `getrandom`.
`ere-compiler-sdk`'s `SdkRustRv64ima` does both steps; for a prebuilt guest object:

```bash
LD_LLD=ld.lld sdk/link.sh <sdk> <guest.a> <guest.elf> [llvm-option...]
```

Options after the output are LLVM options for this guest on this zkVM, passed as `-mllvm`: tuning
that belongs to the guest team, such as the Optuna-tuned set ere's ZisK compiler uses for ethrex
(`ERE_PROFILE=ethrex`, starting `--inline-threshold=4749`). They become part of the link command.

`link.sh` rejects a guest object that does not define `main`, defines an ABI symbol, or needs a
symbol outside the ABI. The guest object is built for plain RV64IM; if the SDK has a
`zkvm.features` file, `link.sh` appends its features to every function's `target-features`
attribute in the guest's bitcode, so the zkVM's extensions are used without a per-zkVM build. (The
attribute replaces the features `ld.lld` would give code generation, so a `-mattr` at link time
has no effect.) It then runs the one fixed command,
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
- `zkvm.features` follows each vendor's own guest builds: OpenVM `+unaligned-scalar-mem`; ZisK
  `+zba,+zbb,+zbkb,+zbs,+unaligned-scalar-mem` (misaligned access also lowers its proving cost);
  SP1 none, because its executor rejects misaligned loads, although the zkvm-standards RISC-V
  target requires `Zicclsm`. The ELF's RISC-V `arch` attribute still reads RV64IM, since it comes
  from module metadata, not from the function attributes.
- The ZisK SDK carries `plugins/zisk-dma`, an LLVM pass plugin that lowers small constant-size
  `memcpy`, `memset` and `memcmp`/`bcmp` (16 to 2047 bytes) at the end of LTO to the inline DMA
  patterns ZisK's transpiler fuses into one `dma_xmem*` operation, as ZisK's own compiler emits them.
  Stock LLVM expands such copies into loads and stores, so the SDK's DMA `memcpy` never sees them.
  On one ethrex block this saves 58k of 714k steps. `build.sh` compiles it with `cmake` against
  `LLVM_BIN`'s LLVM, which must be the LLVM of the `ld.lld` that loads it.
- The SDKs still export a few symbols outside the ABI that vendor assembly references (`__start`
  on OpenVM and SP1, `ZISK_BUMP_HEAP_POS`/`ZISK_BUMP_HEAP_TOP` on ZisK).
