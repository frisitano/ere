//! The zkvm-standards C ABI's I/O and termination, for a staticlib (`staticlib/build.sh`); the
//! `zkvm_*` accelerators are [`crate::zkvm_accelerator`]'s. `_start`, the panic handler and
//! OpenVM's own heap, a private region that `staticlib/zkvm.ld` defines apart from the guest's
//! heap, come from the `openvm` runtime.

use alloc::vec::Vec;
use core::slice;

use ere_platform_core::Platform;
use openvm::platform::rust_rt::terminate;

use crate::OpenVMPlatform;

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
    OpenVMPlatform::write_output(unsafe { slice::from_raw_parts(output, size) });
}

/// Failed termination, with exit code 1 (OpenVM's exit code is an instruction immediate).
#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    terminate::<1>()
}
