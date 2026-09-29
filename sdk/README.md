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
accelerator of the zkvm-standards, plus `abort`, which ends the program as a failed termination: a
`no_std` guest's panic handler needs a function to call, and the standards define none.

The guest's heap is the standard's application heap, `_heap_start` to `_heap_end` in `zkvm.ld`, and
the guest brings its own allocator. Each vendor library allocates from a private region instead:

- ZisK: `ziskos`, built with its `staticlib` feature, allocates from an 8 MiB buffer of its own,
  which each exported function rewinds, since nothing it allocates outlives a call.
- SP1: `sp1-zkvm`'s allocator keeps its heap, from the end of the program's data to SP1's input
  region, and `zkvm.ld` gives the guest 1.5 GiB of the memory below SP1's stack instead.
- OpenVM: its runtime starts its heap at the `_end` symbol and allocates up to the top of memory, so
  `zkvm.ld` places `_end` 64 MiB below the top. The SDK builds OpenVM with its `embedded-alloc` heap,
  patched to `embedded-alloc/`, a bump allocator that each `zkvm_*` accelerator rewinds when it
  returns (`ere-platform-openvm`'s `scoped-heap`), as ZisK's does. The input OpenVM's `read_input`
  reads stays, since no accelerator allocated it.

Everything else in a vendor's library, including the `sys_*` functions upstream's zkVM port of
Rust's `std` calls, is internal. A guest that uses `std` supplies those itself, on these symbols.

## Build an SDK

```bash
sdk/build.sh <openvm|zisk|sp1> <out-dir>
```

Requirements: nightly Rust with `rust-src`, and LLVM tools no older than nightly's LLVM
(`LLVM_BIN`, default Homebrew `llvm@22`).

Each zkVM's directory (`openvm/`, `zisk/`, `sp1/`) holds everything for that zkVM: its linker script
(`zkvm.ld`), ZisK's LTO plugin (`lto-plugin/`), and a vendor crate, a `staticlib` built for the
target spec next to it. That spec is the generic target with only `os`/`vendor` (and, for OpenVM,
`+unaligned-scalar-mem`) changed, so the vendor's cfgs select its accelerated paths, and the crate
is built with the zkVM's `zkvm.features`, as in the vendor's own guest builds. `vendor-archive.sh`
then merges the archive's bitcode into one module, internalizes everything outside the ABI, so the
vendor's `core`, panic handler and allocator cannot collide with the guest's, and compiles that
module into one fat LTO object. A native vendor member that rustc kept out of LTO (OpenVM's
`memcpy`) gets an anchor that `zkvm.ld` names in `EXTERN`, so it is always linked instead of losing
to the guest's weak `compiler_builtins`. `vendor-archive.sh` also drops `noinline` from the ABI
exports, which vendors set for their own builds: across the ABI, as inside a vendor's own guest, the
link's LTO decides what to inline.

## Published SDKs

Pushing a tag `sdk-*` runs `.github/workflows/release-sdk.yml`, which builds the three SDKs (in a
clean Ubuntu 24.04 container with LLVM 22 only, since `bindgen` in vendor build scripts breaks when
another `clang` is on `PATH`) and publishes `sdk-<zkvm>.tar.gz` and `SHA256SUMS` as release assets.
The ZisK plugin in the release is built for Linux x86_64 against LLVM 22.

## Vendor notes

- OpenVM ships no linker script; `openvm/zkvm.ld` follows `openvm-platform`'s memory layout and
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
- `zisk/zkvm.ld` puts `.data` in the same segment as `.bss`: GNU ld emits a declared segment with
  no sections at address 0, which ZisK rejects, and a guest may have no initialized data.
- The ZisK SDK carries `zisk/lto-plugin`, an LLVM pass plugin that applies the rules of ZisK's own
  compiler (the `zisk-dma` LLVM patch of ZisK's Rust toolchain) at the end of LTO: every `memcpy`,
  `memmove`, `memset` with a constant byte, and `memcmp`/`bcmp` longer than 16 bytes, or of runtime
  length, becomes the inline DMA marker sequence ZisK's transpiler fuses into one `dma_xmem*`
  operation, and LLVM's own inline expansion of these operations is capped at two XLEN-sized
  operations. Stock LLVM expands such operations into loads and stores or calls the SDK's `memcpy`:
  without the plugin, BN254 pairing and KZG blocks take about three times as many steps, and
  lowering runtime-length copies too saves a further 8–10% of the proving cost of `modexp`-heavy
  blocks. `build.sh` compiles it with `cmake` against `LLVM_BIN`'s LLVM, which must be the LLVM of
  the `ld.lld` that loads it.
- The SDKs still export a few symbols outside the ABI that vendor assembly references (`__start`
  on OpenVM and SP1, `ZISK_BUMP_HEAP_POS`/`ZISK_BUMP_HEAP_TOP` on ZisK).
