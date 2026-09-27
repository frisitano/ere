//! OpenVM SDK library.
//!
//! - `zkvm_*` come from `ere-platform-openvm`'s `zkvm_accelerator` module.
//! - `_start`, the heap (`sys_alloc_aligned`) and the panic handler come from the `openvm` runtime,
//!   enabled because this library is built for a target with `os = "openvm"`.
//! - The rest of the guest ABI is defined here, after `openvm`'s own `pal_abi` module, which is
//!   only built for OpenVM's forked `std` toolchain.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::slice;

use ere_platform_openvm as _;
use openvm_platform::rust_rt::terminate;
use openvm_riscv_guest::{
    HINT_WORD_BYTES, hint_buffer_bytes, hint_random, raw_print_str_from_bytes,
};

/// Maximum bytes the guest may reveal, as in `ere_platform_openvm::OpenVMPlatform`.
const MAX_OUTPUT_BYTES: usize = 256;

/// Exit code of every abnormal termination; `terminate` takes it as a constant.
const FAILURE: u8 = 1;

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
    assert!(
        size <= MAX_OUTPUT_BYTES,
        "output exceeds {MAX_OUTPUT_BYTES} bytes"
    );
    let output = unsafe { slice::from_raw_parts(output, size) };
    for (index, chunk) in output.chunks(8).enumerate() {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        openvm::io::reveal_u64(u64::from_le_bytes(word), index);
    }
}

#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    terminate::<FAILURE>()
}

/// OpenVM's exit code is an instruction immediate, so every non-zero `code` becomes `1`.
#[unsafe(no_mangle)]
extern "C" fn exit(code: i32) -> ! {
    if code == 0 {
        terminate::<0>()
    } else {
        terminate::<FAILURE>()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sys_panic(msg_ptr: *const u8, len: usize) -> ! {
    raw_print_str_from_bytes(msg_ptr, len);
    terminate::<FAILURE>()
}

/// Writes to stdout and stderr go to the host's stdout; other descriptors are unsupported.
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_write(fd: u32, write_buf: *const u8, nbytes: usize) {
    match fd {
        1 | 2 => raw_print_str_from_bytes(write_buf, nbytes),
        _ => terminate::<FAILURE>(),
    }
}

/// There is no standard input.
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_read(_fd: u32, _recv_buf: *mut u8, _nrequested: usize) -> usize {
    0
}

/// Fills `words` 32-bit words from the host hint stream, as OpenVM's `pal_abi` does (in bytes).
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_rand(recv_buf: *mut u32, words: usize) {
    let nbytes = words * 4;
    if nbytes == 0 {
        return;
    }
    hint_random(nbytes.div_ceil(HINT_WORD_BYTES));
    unsafe { hint_buffer_bytes(recv_buf.cast(), nbytes) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sys_alloc_words(nwords: usize) -> *mut u32 {
    unsafe { openvm_platform::memory::sys_alloc_aligned(nwords * 4, 4).cast() }
}

/// No environment variables are set.
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_getenv(
    _out_words: *mut u32,
    _out_nwords: usize,
    _varname: *const u8,
    _varname_len: usize,
) -> usize {
    usize::MAX
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

#[path = "../../shims/u256.rs"]
mod u256;

/// `zkvm_u256_*`: multiplication, and exponentiation built on it, with OpenVM's Int256 extension;
/// OpenVM has no 256-bit division or modular operation with a runtime modulus, so the rest run in
/// software.
mod u256_ops {
    // Links `openvm-bigint-guest`, which defines the hook below, into the archive.
    use openvm_bigint_guest as _;

    pub use super::u256::sw::{add_mod, div, mul_mod, rem};
    use super::u256::{Limbs, pow_with};

    unsafe extern "C" {
        // `openvm-bigint-guest`'s hook for the Int256 `MUL` instruction, over little-endian bytes.
        fn zkvm_u256_wrapping_mul_impl(result: *mut u8, a: *const u8, b: *const u8);
    }

    pub fn mul(a: &Limbs, b: &Limbs) -> Limbs {
        let mut result = [0; 4];
        unsafe {
            zkvm_u256_wrapping_mul_impl(
                result.as_mut_ptr().cast(),
                a.as_ptr().cast(),
                b.as_ptr().cast(),
            )
        };
        result
    }

    pub fn pow(base: &Limbs, exponent: &Limbs) -> Limbs {
        pow_with(mul, base, exponent)
    }
}
