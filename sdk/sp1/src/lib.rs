//! SP1 SDK library. Everything comes from SP1's `libzkevm`, built for a target with
//! `os = "zkvm"` and `vendor = "succinct"` so `sp1-zkvm` and SP1's patched crypto crates select
//! their syscall paths. `libzkevm` exports `_start`, `read_input`, `write_output`, every `zkvm_*`
//! symbol, `abort` and the heap (`sys_alloc_aligned`).
//!
//! Some of `libzkevm`'s dependencies need `std`, which the stock toolchain builds from source
//! (`-Zbuild-std=std`) with upstream's `target_os = "zkvm"` port. `std` also supplies the panic
//! handler, which `vendor-archive.sh` keeps internal.

use zkevm as _;
