//! Negative control: `zkvm_keccak256` and `zkvm_sha256` in plain software (RustCrypto), with no
//! zkVM awareness. Linked ahead of a vendor archive whose own two symbols are internalized, it
//! shows the instruction count of a build that silently lost acceleration.

#![no_std]

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

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
