# Stock-link spike: one guest, three zkVMs, plain `cargo build` plus a vendor SDK

Spike, not production code. It tests whether a guest can be built with a stock nightly Rust
toolchain and get its zkVM accelerators and runtime only at link time, from a vendor archive.

The guest is built with an ordinary `cargo build --release`. The only zkVM-specific input is an
SDK directory with two files, the shape the zkvm-standards "Static Library and Linker Script"
proposal describes:

```text
out/sdk/<zkvm>/
├── libzkvm.a   vendor archive: one LLVM bitcode module exporting only the standard symbols
└── zkvm.ld     vendor linker script; `INPUT(-lzkvm)` pulls the archive into the link
```

```bash
cd guest
cargo +nightly build --release \
    --config 'target.riscv64ima-unknown-none-elf.rustflags = ["-Clink-arg=-L<sdk>", "-Clink-arg=-T<sdk>/zkvm.ld"]'
```

Everything else is zkVM independent and lives in `guest/.cargo/config.toml` (target spec,
`build-std`, `-Clinker-plugin-lto`) and `guest/Cargo.toml` (`lto = "fat"`).

## What is built

- `guest/`: the only guest. It depends on `zkvm-interface` alone (`extern "C"` declarations
  from `zkvm_accelerators.h` and `zkvm_io.h`) and exports `int main(void)`. It hashes its input
  with `zkvm_keccak256` and `zkvm_sha256` and writes both digests with `write_output`. The source
  and dependency graph are the same for every zkVM:

  ```text
  stock-link-guest
  └── zkvm-interface (eth-act/zkvm-standards @ 282cd35)
  ```

- One vendor crate per zkVM. Each is a `staticlib` built with stock nightly and
  `-Clinker-plugin-lto`, so the archive holds LLVM bitcode:

  | zkVM   | Crate     | Built from                                             | Target `os`/`vendor` |
  | ------ | --------- | ------------------------------------------------------ | -------------------- |
  | OpenVM | `openvm/` | `ere-platform-openvm` (`zkvm_accelerator.rs`) + `openvm` runtime | `openvm` / none |
  | ZisK   | `zisk/`   | `ziskos` 1.2.0-alpha                                   | `zkvm` / `zisk`      |
  | SP1    | `sp1/`    | `libzkevm` from SP1 v6.6.0 (upstream, no fork)         | `zkvm` / `succinct`  |

  Each target spec (in `targets/`) is the stock `riscv64ima` spec with only `os`/`vendor`
  changed. That changes the vendor's cfgs (`openvm_intrinsics`, `zisk_guest`,
  `target_os = "zkvm"`) so they select the accelerated paths, while the LLVM triple stays the
  same, so the bitcode links with the stock guest.

- `vendor-archive.sh`: merges the archive's bitcode into one module and internalizes every symbol
  except `exports.txt` (the 20 `zkvm_*` functions, `read_input`, `write_output`, `_start`,
  `__start`). The vendor's copy of `core`/`std`, its panic handler and its allocator can then no
  longer collide with the guest's. The first link failed on a duplicate
  `__rustc::rust_begin_unwind` until this step existed.

- Linker scripts in `linker/`: ZisK's own (`zisk.ld`), SP1's from its `zkevm/` SDK
  (`sp1.ld`), and `openvm.ld`, written here because OpenVM ships none. It reproduces the layout
  ere gets from `-Ttext=0x00200800`. `run-all.sh` prepends `INPUT(-lzkvm)` to each script to
  make the SDK's `zkvm.ld`.

- `build-guest.sh <sdk-dir> <elf>`: the `cargo build` above. Cargo does not track the SDK files,
  so it cleans the guest package first to force a relink.

- `software/`: negative control. It exports only `zkvm_keccak256` and `zkvm_sha256`, implemented
  with RustCrypto. The control SDK (`out/sdk/<zkvm>-software`) holds the vendor archive
  re-internalized without those two symbols (`exports-without-hashes.txt`) plus the software
  archive (`INPUT(-lzkvm -lzkvm_software)`), so the link takes the software hashes and
  everything else from the vendor.

- `link-guest.sh`: links with `-l<vendor>` and without rustc-side LTO. It exists only to
  reproduce the split and thin variants in `compare-lto.sh`.

- `runner/`: host runner. It executes an ELF on the zkVM's own executor (OpenVM interpreter,
  `ziskemu`, SP1 `MinimalExecutor`), checks the output against host keccak256/sha256 and prints
  the retired instruction count. There is one cargo feature per zkVM.

## Results

`./run-all.sh` on 2026-09-25 (macOS arm64, nightly 1.96 / LLVM 22.1.0, Homebrew LLVM 22.1.8),
both columns built with plain `cargo build` and full LTO. Counts are instructions retired by
each zkVM's executor, not proving cost. Every run's output matched the host digests.

| zkVM   | Input (bytes) | Accelerated (instr.) | Software control (instr.) | Ratio |
| ------ | ------------: | -------------------: | ------------------------: | ----: |
| OpenVM |             0 |                  747 |                    11,669 | 15.6x |
| OpenVM |         1,000 |                2,292 |                   122,286 | 53.4x |
| OpenVM |        10,000 |               16,084 |                 1,160,269 | 72.1x |
| ZisK   |             0 |                  925 |                    11,870 | 12.8x |
| ZisK   |         1,000 |                4,049 |                   122,407 | 30.2x |
| ZisK   |        10,000 |               33,416 |                 1,160,314 | 34.7x |
| SP1    |             0 |                3,015 |                    13,368 |  4.4x |
| SP1    |         1,000 |               14,478 |                   123,909 |  8.6x |
| SP1    |        10,000 |              116,528 |                 1,161,881 | 10.0x |

Static evidence, checked on the split build (below), where `zkvm_keccak256` and `zkvm_sha256`
are still separate functions; in the full-LTO build they are inlined into `main`:

- OpenVM: `zkvm_keccak256` and `zkvm_sha256` contain custom-0 (`opcode 0x0b`) instructions.
- ZisK: `zkvm_keccak256` writes CSR `0x800` (keccak-f); `zkvm_sha256` writes CSR `0x805`
  (sha256-f).
- SP1: `zkvm_keccak256` issues `ecall` with `t0 = 0x00010109` (`KECCAK_PERMUTE`);
  `zkvm_sha256` calls the patched `compress256`, which issues the `SHA_EXTEND`/`SHA_COMPRESS`
  ecalls.

LTO keeps only what the guest calls: the full-LTO OpenVM guest is 22 KB, without the other 18
accelerators.

## LTO mode: split, thin, full

`./compare-lto.sh` (after `run-all.sh`) builds the same guest and vendor code three ways.
Instruction counts; every output matched the host:

| zkVM   | Input (bytes) |   Split |    Thin |    Full | Full vs split |
| ------ | ------------: | ------: | ------: | ------: | ------------: |
| OpenVM |             0 |   1,506 |   1,474 |     747 |        -50.4% |
| OpenVM |         1,000 |   3,049 |   3,465 |   2,292 |        -24.8% |
| OpenVM |        10,000 |  16,841 |  21,487 |  16,084 |         -4.5% |
| ZisK   |             0 |   1,022 |   1,083 |     925 |         -9.5% |
| ZisK   |         1,000 |   4,952 |   5,014 |   4,049 |        -18.2% |
| ZisK   |        10,000 |  41,909 |  41,971 |  33,416 |        -20.3% |
| SP1    |             0 |   3,140 |   3,253 |   3,015 |         -4.0% |
| SP1    |         1,000 |  14,600 |  14,724 |  14,478 |         -0.8% |
| SP1    |        10,000 | 116,650 | 116,906 | 116,528 |         -0.1% |

- Split: vendor module without a ThinLTO summary, guest ThinLTO (what rustc emits under
  `-Clinker-plugin-lto` when the profile has no `lto`). lld optimizes the two in separate partitions, and ThinLTO cannot
  import from a module without a summary, so inline remarks report the vendor functions as
  "NoDefinition" for `main`. This was the first version of the spike.
- Thin: vendor module written with `--thinlto-bc`. The guest imports and inlines the small
  wrappers, but the vendor internals are optimized module by module; OpenVM loses ground at
  larger inputs.
- Full: `build-guest.sh`, plain `cargo build` with `lto = "fat"` and `-Clinker-plugin-lto`.
  rustc runs fat LTO over the guest and `core` and hands the linker one bitcode module without a
  ThinLTO summary, and lld's LTO (`-plugin-opt=O3`) optimizes it together with the vendor module.
  This is the fastest and smallest on all three zkVMs, and the inliner inlined
  `zkvm_keccak256` and `zkvm_sha256` into `main` on cost alone, without `always_inline`.

SP1 gains least because its syscalls are primitives (keccak-f, SHA extend/compress), so most
instructions are in the sponge and padding loops, which LTO mode barely changes.

## Findings

1. Vendor archives must export only the standard symbols. Two Rust staticlibs each carry a panic
   handler (and `core`, and an allocator); the duplicate `rust_begin_unwind` fails the link.
   Merging to one bitcode module and internalizing fixes this without giving up LTO.
2. The vendor's cfgs must be fixed when the archive is built, never when the guest is built.
   Before this, the guest's target decided whether the vendor code was accelerated. Setting
   `os`/`vendor` on the vendor's own target spec is enough for all three.
3. The vendor build must apply the vendor's patch set. Without SP1's `sha2` patch, `sp1-zkvm`'s
   public-values hasher (in `write_output` and `syscall_halt`) linked an unpatched software
   `sha2`, even though `zkvm_sha256` itself was accelerated.
4. SP1's `libzkevm` needs `std` through transitive dependencies. Stock `-Zbuild-std=std` works,
   using upstream's `target_os = "zkvm"` port, whose `sys_*` functions `sp1-zkvm` defines. No
   Succinct toolchain was needed.
5. Cargo does not track an external archive. `build-guest.sh` and `link-guest.sh` clean the guest
   package so it relinks every time; an earlier stale ELF made SP1's runtime look unaccelerated.
6. Bitcode archives tie the vendor to the linker's LLVM major version (22 here). A vendor that
   cannot guarantee that could ship native objects instead, losing only cross-boundary inlining.
7. The best code needs one full-LTO module across guest and vendor (previous section). Plain
   cargo does this with `lto = "fat"` and `-Clinker-plugin-lto`, provided the vendor archive
   holds a plain bitcode module. ThinLTO bitcode also links but optimizes worse here.
8. The vendor linker script can name the vendor archive (`INPUT(-lzkvm)`), so the guest-side
   link configuration is the same for every zkVM: `-T <sdk>/zkvm.ld` and `-L <sdk>`.

## Open

- Only `zkvm_keccak256` and `zkvm_sha256` are exercised. The other 18 symbols are exported by
  each archive but not linked or run here; ere's `ZkvmInterfaceProgram` vectors should run
  through this path.
- The guest does not allocate. A guest heap and the vendor's internal heap would both start at
  the end of `.bss` (`_end`, `_heap_bottom`), so heap ownership has to be specified before a real
  guest (reth, ethrex) can use this.
- OpenVM's `openvm` runtime calls `main` as `fn()`, so the `int` return is ignored, unlike ZisK
  and SP1, which pass it to their exit syscalls.
- The guest's panic handler loops. The standard has no abort/exit symbol for it to call.
- Non-standard symbols (`native_keccak256`, ZisK's `mul_mod_bytes256_c`) are not covered.

## Reproduce

```bash
# ZisK's host emulator links a C++ library whose Makefile expects full Xcode; with only the
# Command Line Tools, point it at their SDK and use Apple clang.
MAKEFLAGS="SDKPATH=$(xcrun --show-sdk-path)" CC=/usr/bin/clang CXX=/usr/bin/clang++ ./run-all.sh
./compare-lto.sh
```
