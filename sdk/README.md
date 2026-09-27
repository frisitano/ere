# zkVM SDKs

A guest built once, naming no zkVM, gets its runtime and accelerators at link time from a zkVM SDK.
An SDK is a directory with the two files of the zkvm-standards "Static Library and Linker Script"
proposal, plus optional link settings:

```text
<sdk>/
├── libzkvm.a       one LLVM bitcode module exporting exactly the guest ABI (`abi.txt`)
├── zkvm.ld         the vendor linker script; `INPUT(-lzkvm)` pulls the archive into the link
├── zkvm.features   optional: ISA extensions beyond RV64IM, added to the guest's code at link time
├── zkvm.llvm-args  optional: LLVM options the vendor's own build uses for every guest, one per line
├── libzkvm_software.a  every `zkvm_*` accelerator in plain RISC-V, linked only if selected
└── zkvm-lto-plugin.so  optional: LLVM pass plugin run at the end of LTO (native, per host)
```

Until zkVM teams publish SDKs, `build.sh` builds them from pinned vendor sources with stock nightly
Rust.

## Build an SDK

```bash
sdk/build.sh <openvm|zisk|sp1> <out-dir>
```

Requirements: nightly Rust with `rust-src`, and LLVM tools no older than nightly's LLVM
(`LLVM_BIN`, default Homebrew `llvm@22`).

Each vendor crate (`openvm/`, `zisk/`, `sp1/`) is a `staticlib` built for its own target in
`targets/`: the generic target with only `os`/`vendor` (and, for OpenVM, `+unaligned-scalar-mem`)
changed, so the vendor's cfgs select its accelerated paths, and with the zkVM's `zkvm.features`, as
in the vendor's own guest builds. `vendor-archive.sh` then merges the
archive's bitcode into one module and internalizes everything outside the ABI, so the vendor's
`core`, panic handler and allocator cannot collide with the guest's. A native vendor member that
rustc kept out of LTO (OpenVM's `memcpy`) gets an anchor that `zkvm.ld` names in `EXTERN`, so it is
always linked instead of losing to the guest's weak `compiler_builtins`. `vendor-archive.sh` also
drops `noinline` from the ABI exports: ZisK marks `sys_alloc_aligned` `#[inline(never)]` for its own
builds, whose `std` allocates without calling it, and a generic guest's `std` calls it on every
allocation. Across the ABI, as inside a vendor's own guest, the link's LTO decides what to inline.

Every SDK also carries `libzkvm_software.a`: every `zkvm_*` accelerator in plain RISC-V, from
`software/` (`revm-precompile`'s pure-Rust backends and `ruint`), compiled with the zkVM's features.
It is linked only when `link.sh --software <pattern>,...` selects accelerators (a symbol, or a prefix
ending in `*`: `zkvm_u256_*`, or `zkvm_*` for all). `link.sh` then internalizes the selected symbols
in the vendor module and exports only them from the software module, which takes seconds, so each
configuration is one link from the same SDK and guest object. That measures what each accelerator
is worth, down to all-software as the acceleration check's negative control. The vendor's own
internal uses of an accelerator keep the vendor's code.

## 256-bit arithmetic

`zkvm_u256_mul`, `_div`, `_mod`, `_addmod`, `_mulmod` and `_exp` compute the EVM opcodes of those
names: wrapping modulo 2^256, and zero for a zero divisor or modulus. Their operands are
`uint64_t[4]` in little-endian limb order, 8-byte aligned, which is how EVM implementations hold
their stack words and how every zkVM's 256-bit precompile reads its operands, so a guest passes
pointers to its stack slots and nothing is converted. `result` may alias an input. The names follow
the zkvm-standards U256 draft (`zkvm_u256.h`), whose operands are big-endian bytes instead.
`shims/u256.rs` holds the exports, a software reference on `ruint`, and a square-and-multiply
`exp` on a zkVM's multiplication:

| Function | OpenVM | SP1 | ZisK |
| --- | --- | --- | --- |
| `mul` | Int256 `MUL` | `UINT256_MUL` (modulus 0) | `arith256` |
| `div`, `mod` | software | software | `arith256` (hinted, verified) |
| `addmod` | software | software | `arith256_mod` |
| `mulmod` | software | `UINT256_MUL` | `arith256_mod` |
| `exp` | on Int256 `MUL` | on `UINT256_MUL` | `ziskos` `wrapping_pow256` |

## Published SDKs

Guest CI does not build SDKs. Pushing a tag `sdk-*` runs `.github/workflows/release-sdk.yml`, which
builds the three SDKs (in a clean Ubuntu 24.04 container with LLVM 22 only, since `bindgen` in
vendor build scripts breaks when another `clang` is on `PATH`) and publishes, as
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

LLVM options reach the link as `-mllvm` from two places. The SDK's `zkvm.llvm-args` holds what the
vendor's own build passes for every guest (SP1: `-misched-prera-direction=bottomup` and
`-misched-postra-direction=bottomup`, from `cargo prove build`). Options after the output are this
guest's tuning on this zkVM, which belongs to the guest team, such as the Optuna-tuned set ere's ZisK
compiler uses for ethrex (`ERE_PROFILE=ethrex`, starting `--inline-threshold=4749`). Guest options
come last, so they can override the vendor's. Together, the two reproduce the optimization settings
of ere's `rust-customized` compilers exactly. Both become part of the link command.

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
  target requires `Zicclsm`. ZisK's own toolchain leaves out Zba; the SDK keeps it, since without
  it the linked guests take more steps.
- The ZisK SDK carries `plugins/zisk-dma`, an LLVM pass plugin that lowers small constant-size
  `memcpy`, `memset` and `memcmp`/`bcmp` (16 to 2047 bytes) at the end of LTO to the inline DMA
  patterns ZisK's transpiler fuses into one `dma_xmem*` operation, as ZisK's own compiler emits them.
  Stock LLVM expands such copies into loads and stores, so the SDK's DMA `memcpy` never sees them.
  On one ethrex block this saves 58k of 714k steps. `build.sh` compiles it with `cmake` against
  `LLVM_BIN`'s LLVM, which must be the LLVM of the `ld.lld` that loads it.
- The SDKs still export a few symbols outside the ABI that vendor assembly references (`__start`
  on OpenVM and SP1, `ZISK_BUMP_HEAP_POS`/`ZISK_BUMP_HEAP_TOP` on ZisK).
