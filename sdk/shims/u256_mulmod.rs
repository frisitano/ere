//! `zkvm_u256_mulmod` of the zkvm-standards U256 interface draft (`zkvm_u256.h`) in plain RISC-V,
//! for SDKs whose zkVM has no modular multiplication with a runtime modulus. Included by path.

use ruint::aliases::U256;

/// `result = a * b mod n` over 32-byte big-endian words, with the full 512-bit product; zero if
/// `n` is zero (EVM `MULMOD`). `result` may alias an input.
#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_mulmod(
    a: *const [u8; 32],
    b: *const [u8; 32],
    n: *const [u8; 32],
    result: *mut [u8; 32],
) -> i32 {
    let (a, b, n) = unsafe {
        (U256::from_be_bytes(*a), U256::from_be_bytes(*b), U256::from_be_bytes(*n))
    };
    unsafe { *result = a.mul_mod(b, n).to_be_bytes() };
    0
}
