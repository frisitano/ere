# zkVM SDKs

A guest built once, naming no zkVM, gets its runtime and accelerators at link time from a zkVM SDK.
An SDK is a directory with the two files of the zkvm-standards "Static Library and Linker Script"
standard, plus two optional ones:

```text
<sdk>/
├── libzkvm.a           fat LTO objects exporting exactly the guest ABI (`abi.txt`)
├── zkvm.ld             the vendor linker script; `INPUT(-lzkvm)` pulls the archive into the link
├── zkvm.features       optional: ISA extensions beyond RV64IM, added to the guest's code at link time
└── zkvm-lto-plugin.so  optional: LLVM pass plugin run at the end of LTO (native, per host)
```

`libzkvm.a` holds each function twice: native RV64IM code, with the zkVM's extensions, which any
linker can use, and the same code as LLVM bitcode in the object's `.llvm.lto` section, which
`ld.lld --fat-lto-objects` optimizes together with the guest's bitcode, so accelerator calls inline.
It holds no compiler runtime builtins (`__muldi3` and the like): the guest's toolchain supplies them,
so a link holds one copy of each.

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
in the vendor's own guest builds. `vendor-archive.sh` then merges the archive's bitcode into one
module, internalizes everything outside the ABI, so the vendor's `core`, panic handler and allocator
cannot collide with the guest's, and compiles that module into one fat LTO object. A native vendor
member that rustc kept out of LTO (OpenVM's `memcpy`) gets an anchor that `zkvm.ld` names in
`EXTERN`, so it is always linked instead of losing to the guest's weak `compiler_builtins`.
`vendor-archive.sh` also drops `noinline` from the ABI exports: ZisK marks `sys_alloc_aligned`
`#[inline(never)]` for its own builds, whose `std` allocates without calling it, and a generic
guest calls it on every allocation. Across the ABI, as inside a vendor's own guest, the link's LTO
decides what to inline.

## The guest ABI

`abi.txt` lists what a guest may take from an SDK: `_start`, `read_input`, `write_output` and every
`zkvm_*` accelerator of the zkvm-standards, plus, until the standards cover them:

- `abort`, which ends the program as a failed termination. A `no_std` guest's panic handler needs a
  function to call, and the standards define none.
- `sys_alloc_aligned(bytes, align)`, the zkVM's heap, which never frees. zkVM libraries allocate from
  the same memory the standards' `_heap_start`/`_heap_end` give the application, so until they keep
  their scratch memory apart, a guest allocates through them.
- `exit` and the other `sys_*` functions, which upstream's zkVM port of `std` calls.

## Build and link a guest

The guest is a `staticlib` defining `int main(void)`, built with `lto = "fat"` and
`-Clinker-plugin-lto` for one of two generic targets (`crates/compiler/sdk`):

- `riscv64im-unknown-none-elf`, for a `no_std` guest (`SdkRustRv64im`): the stock bare-metal spec
  with compare-and-swap, which `alloc::sync` needs, built with stable Rust (`RUSTC_BOOTSTRAP=1` for
  `-Zbuild-std=core,alloc`). A guest runs on one thread, so `-Cpasses=lower-atomic` lowers its
  atomics to plain loads and stores. `ere-platform-zkvm` is its runtime: a global allocator on
  `sys_alloc_aligned` and a panic handler that calls `abort`. Such a guest needs only the standard
  symbols, `abort` and `sys_alloc_aligned`.
- `riscv64ima-unknown-zkvm-elf`, for a guest that uses `std` (`SdkRustRv64ima`): the same ISA with
  `os = "zkvm"`, built with the pinned nightly, where `std` calls the SDK's `sys_*` functions.

`ere-compiler-sdk` builds the guest object and links it; for a prebuilt guest object:

```bash
LD_LLD=ld.lld sdk/link.sh <sdk> <guest.a> <guest.elf> [llvm-option...]
```

Options after the output reach the link as `-mllvm`: this guest's tuning on this zkVM, such as the
scheduling direction SP1's `cargo prove build` uses for every guest
(`-misched-prera-direction=bottomup -misched-postra-direction=bottomup`) or the Optuna-tuned set
ere's ZisK compiler uses for ethrex (`ERE_PROFILE=ethrex`, starting `--inline-threshold=4749`).
They become part of the link command.

`link.sh` rejects a guest object that does not define `main`, defines an ABI symbol, or needs a
symbol outside the ABI. The guest object is built for plain RV64IM; if the SDK has a
`zkvm.features` file, `link.sh` appends its features to every function's `target-features`
attribute in the guest's bitcode, so the zkVM's extensions are used without a per-zkVM build. (The
attribute replaces the features `ld.lld` would give code generation, so a `-mattr` at link time
has no effect.) It then runs the one fixed command,
`ld.lld -T <sdk>/zkvm.ld -L <sdk> --gc-sections --fat-lto-objects --lto-O3 -o <guest.elf> <guest.a>`.

A linker without LTO support, such as GNU ld, links the archive's native code instead, as for a C
guest: `ld -T <sdk>/zkvm.ld -L <sdk> main.o -lgcc`.

## Published SDKs

Guest CI does not build SDKs. Pushing a tag `sdk-*` runs `.github/workflows/release-sdk.yml`, which
builds the three SDKs (in a clean Ubuntu 24.04 container with LLVM 22 only, since `bindgen` in
vendor build scripts breaks when another `clang` is on `PATH`) and publishes, as release assets,
`sdk-<zkvm>.tar.gz`, `ere-link-tools.tar.gz` (`link.sh`, `abi.txt`, the two guest target specs and
their pinned toolchains, `rust-toolchain-no-std` and `rust-toolchain`) and `SHA256SUMS`, so one
release pins the vendor runtimes, the link tools and the guest toolchains together. The ZisK plugin
in the release is built for Linux x86_64 against LLVM 22.

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
- `linker/zisk.ld` puts `.data` in the same segment as `.bss`: GNU ld emits a declared segment with
  no sections at address 0, which ZisK rejects, and a guest may have no initialized data.
- The ZisK SDK carries `plugins/zisk-dma`, an LLVM pass plugin that lowers small constant-size
  `memcpy`, `memset` and `memcmp`/`bcmp` (16 to 2047 bytes) at the end of LTO to the inline DMA
  patterns ZisK's transpiler fuses into one `dma_xmem*` operation, as ZisK's own compiler emits them.
  Stock LLVM expands such copies into loads and stores, so the SDK's DMA `memcpy` never sees them.
  Without it, ethrex takes about 10% more steps on devnet blocks. `build.sh` compiles it with
  `cmake` against `LLVM_BIN`'s LLVM, which must be the LLVM of the `ld.lld` that loads it.
- The SDKs still export a few symbols outside the ABI that vendor assembly references (`__start`
  on OpenVM and SP1, `ZISK_BUMP_HEAP_POS`/`ZISK_BUMP_HEAP_TOP` on ZisK).
