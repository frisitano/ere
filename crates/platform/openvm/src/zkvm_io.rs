//! The [zkvm-standards] I/O interface (`zkvm_io.h`) on OpenVM, which [`crate::OpenVMPlatform`]'s
//! [`Platform`](ere_platform_core::Platform) methods call through their defaults.
//!
//! [zkvm-standards]: https://github.com/eth-act/zkvm-standards

use alloc::vec::Vec;
use core::slice;

use openvm::platform::rust_rt::terminate;

use crate::platform::MAX_OUTPUT_BYTES;

/// Reads the whole input, which stays allocated for the rest of the program.
#[unsafe(no_mangle)]
unsafe extern "C" fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) {
    let input: &'static [u8] = Vec::leak(openvm::io::read_vec());
    unsafe {
        *buf_ptr = input.as_ptr();
        *buf_size = input.len();
    }
}

/// Reveals `size` bytes from `output`, 8 at a time. At most [`MAX_OUTPUT_BYTES`] may be revealed,
/// and output shorter than that is padded to it.
#[unsafe(no_mangle)]
unsafe extern "C" fn write_output(output: *const u8, size: usize) {
    assert!(
        size <= MAX_OUTPUT_BYTES,
        "Maximum output size is {MAX_OUTPUT_BYTES} bytes, got {size} bytes",
    );
    let output = unsafe { slice::from_raw_parts(output, size) };
    for (index, chunk) in output.chunks(8).enumerate() {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        openvm::io::reveal_u64(u64::from_le_bytes(word), index);
    }
}

/// Failed termination, with exit code 1 (OpenVM's exit code is an instruction immediate).
#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    terminate::<1>()
}
