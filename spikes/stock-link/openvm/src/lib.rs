//! OpenVM vendor library: `zkvm_accelerators.h` and `zkvm_io.h` symbols plus the machine runtime.
//!
//! - `zkvm_*` come from `ere-platform-openvm`'s `zkvm_accelerator` module.
//! - `_start`, the panic handler and the heap come from the `openvm` crate, whose runtime is
//!   enabled because this library is built for a target with `os = "openvm"`.
//! - `read_input` and `write_output` are defined here.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::slice;

use ere_platform_openvm as _;

/// Maximum bytes the guest may reveal, see `ere_platform_openvm::OpenVMPlatform`.
const MAX_OUTPUT_BYTES: usize = 256;

#[unsafe(no_mangle)]
unsafe extern "C" fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) {
    let input: &'static [u8] = Vec::leak(openvm::io::read_vec());
    unsafe {
        *buf_ptr = input.as_ptr();
        *buf_size = input.len();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn write_output(output: *const u8, size: usize) {
    assert!(size <= MAX_OUTPUT_BYTES);
    let output = unsafe { slice::from_raw_parts(output, size) };
    for (index, chunk) in output.chunks(8).enumerate() {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        openvm::io::reveal_u64(u64::from_le_bytes(word), index);
    }
}
