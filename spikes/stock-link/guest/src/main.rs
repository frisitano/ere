//! zkVM-agnostic guest. It names no zkVM crate: `zkvm_*`, `read_input` and `write_output` are
//! `extern "C"` declarations from `zkvm-interface`, resolved when the vendor library is linked.
//!
//! Output is `keccak256(input) || sha256(input)`.

#![no_std]
#![no_main]

use core::{ptr, slice};

use zkvm_interface::{
    read_input, write_output, zkvm_keccak256, zkvm_keccak256_hash, zkvm_sha256, zkvm_sha256_hash,
};

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let (mut data, mut len) = (ptr::null(), 0);
    unsafe { read_input(&mut data, &mut len) };
    let input = unsafe { slice::from_raw_parts(data, len) };

    let mut keccak = zkvm_keccak256_hash { data: [0; 32] };
    let mut sha256 = zkvm_sha256_hash { data: [0; 32] };
    unsafe {
        assert_eq!(zkvm_keccak256(input.as_ptr(), input.len(), &mut keccak), 0);
        assert_eq!(zkvm_sha256(input.as_ptr(), input.len(), &mut sha256), 0);
    }

    let mut output = [0u8; 64];
    output[..32].copy_from_slice(&keccak.data);
    output[32..].copy_from_slice(&sha256.data);
    unsafe { write_output(output.as_ptr(), output.len()) };
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
