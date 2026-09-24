//! Negative control: the OpenVM runtime with `zkvm_keccak256` and `zkvm_sha256` in plain software
//! (RustCrypto, no OpenVM intrinsics). Linked the same way as `zkvm-openvm`, it shows what the
//! instruction count looks like when acceleration is missing.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::slice;

use sha2::Digest;
use zkvm_interface::{zkvm_keccak256_hash, zkvm_sha256_hash, zkvm_status};

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_keccak256(
    data: *const u8,
    len: usize,
    output: *mut zkvm_keccak256_hash,
) -> zkvm_status {
    let data = unsafe { slice::from_raw_parts(data, len) };
    unsafe { (*output).data = sha3::Keccak256::digest(data).into() };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_sha256(
    data: *const u8,
    len: usize,
    output: *mut zkvm_sha256_hash,
) -> zkvm_status {
    let data = unsafe { slice::from_raw_parts(data, len) };
    unsafe { (*output).data = sha2::Sha256::digest(data).into() };
    0
}

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
    let output = unsafe { slice::from_raw_parts(output, size) };
    for (index, chunk) in output.chunks(8).enumerate() {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        openvm::io::reveal_u64(u64::from_le_bytes(word), index);
    }
}
