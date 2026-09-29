# zkVM SDKs

A zkVM SDK is what a guest program that names no zkVM is linked against to run on that zkVM: the
two files of the zkvm-standards "Static Library and Linker Script" standard, plus two optional ones.

```text
<sdk>/
├── libzkvm.a           fat LTO objects exporting exactly the guest ABI (`abi.txt`)
├── zkvm.ld             the vendor linker script; `INPUT(-lzkvm)` pulls the archive into the link
├── zkvm.features       optional: ISA extensions beyond RV64IM, for the guest's code at link time
└── zkvm-lto-plugin.so  optional: LLVM pass plugin for the end of LTO (native, per host)
```

`libzkvm.a` holds each function twice: native RV64IM code, with the zkVM's extensions, which any
linker can use, and the same code as LLVM bitcode in the object's `.llvm.lto` section, which
`ld.lld --fat-lto-objects` optimizes together with a guest's bitcode, so accelerator calls inline.
It holds no compiler runtime builtins (`__muldi3` and the like): the guest's toolchain supplies them,
so a link holds one copy of each.

Until zkVM teams publish SDKs, `build.sh` builds them from pinned vendor sources with stock nightly
Rust. Building and linking guests is left to the guest side.

## The guest ABI

`abi.txt` lists what an SDK exports: `_start`, `read_input`, `write_output` and every `zkvm_*`
accelerator of the zkvm-standards, plus, until the standards cover them:

- `abort`, which ends the program as a failed termination. A `no_std` guest's panic handler needs a
  function to call, and the standards define none.
- `sys_alloc_aligned(bytes, align)`, the zkVM's heap, which never frees. zkVM libraries allocate from
  the same memory the standards' `_heap_start`/`_heap_end` give the application, so until they keep
  their scratch memory apart, a guest allocates through them.

Everything else in a vendor's library, including the `sys_*` functions upstream's zkVM port of
Rust's `std` calls, is internal. A guest that uses `std` supplies those itself, on these symbols.

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

## Published SDKs

Pushing a tag `sdk-*` runs `.github/workflows/release-sdk.yml`, which builds the three SDKs (in a
clean Ubuntu 24.04 container with LLVM 22 only, since `bindgen` in vendor build scripts breaks when
another `clang` is on `PATH`) and publishes `sdk-<zkvm>.tar.gz` and `SHA256SUMS` as release assets.
The ZisK plugin in the release is built for Linux x86_64 against LLVM 22.

## Vendor notes

- OpenVM ships no linker script; `linker/openvm.ld` follows `openvm-platform`'s memory layout and
  declares `PHDRS`, so the ELF headers are not loaded inside the stack below `0x0020_0800`.
- OpenVM's `openvm-mem` needs `+unaligned-scalar-mem`; built without it, it stops OpenVM's
  executor.
- SP1's `libzkevm` v6.6.0 does not follow the standard encoding for `zkvm_bls12_*` and
  `zkvm_ripemd160`. The SDK pins the fix ere's SP1 test guests use until
  succinctlabs/sp1#2865 is released.
- SP1's own build (`cargo prove build`) compiles every guest with
  `-misched-prera-direction=bottomup -misched-postra-direction=bottomup`; a guest link that matches
  it passes them to LTO.
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
