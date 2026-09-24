//! SP1 vendor library: everything comes from SP1's `libzkevm`, built for a target with
//! `os = "zkvm"` and `vendor = "succinct"` so `sp1-zkvm` and SP1's patched crypto crates select
//! their syscall paths.
//!
//! Some of `libzkevm`'s dependencies need `std`. The stock toolchain builds it from source with
//! `-Zbuild-std=std`, using upstream's `target_os = "zkvm"` port, whose `sys_*` functions
//! `sp1-zkvm` defines. `std` also supplies the panic handler, which `vendor-archive.sh` keeps
//! internal to the archive.

use zkevm as _;
