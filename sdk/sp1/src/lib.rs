//! SP1 SDK library. Everything comes from SP1's `libzkevm`, built for a target with
//! `os = "zkvm"` and `vendor = "succinct"` so `sp1-zkvm` and SP1's patched crypto crates select
//! their syscall paths. `libzkevm` exports `_start`, `read_input`, `write_output`, every `zkvm_*`
//! symbol, `abort`, `exit` and most of the `sys_*` functions.
//!
//! Some of `libzkevm`'s dependencies need `std`, which the stock toolchain builds from source
//! (`-Zbuild-std=std`) with upstream's `target_os = "zkvm"` port. `std` also supplies the panic
//! handler, which `vendor-archive.sh` keeps internal.
//!
//! The `sys_*` functions `libzkevm` does not export are defined here.

use zkevm as _;

/// There is no standard input.
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_read(_fd: u32, _recv_buf: *mut u8, _nrequested: usize) -> usize {
    0
}

/// There are no program arguments.
#[unsafe(no_mangle)]
extern "C" fn sys_argc() -> usize {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sys_argv(
    _out_words: *mut u32,
    _out_nwords: usize,
    _arg_index: usize,
) -> usize {
    0
}
