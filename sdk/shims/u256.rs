//! 256-bit arithmetic accelerators (`zkvm_u256_*`), over little-endian limbs: a value is a
//! `uint64_t[4]`, least significant limb first, 8-byte aligned. That is how EVM implementations
//! hold their stack words (`ruint`, `ethereum_types`) and how every zkVM's 256-bit precompile reads
//! its operands, so neither side converts. Semantics are the EVM opcodes': wrapping modulo 2^256,
//! and zero for a zero divisor or modulus. `result` may alias any input.
//!
//! Included by path. The including crate provides `u256_ops` with `mul`, `div`, `rem`, `add_mod`,
//! `mul_mod` and `pow` over [`Limbs`], using its zkVM's precompiles where it has them and the
//! software reference in [`sw`] otherwise.

use super::u256_ops as ops;

pub type Limbs = [u64; 4];

/// Plain RISC-V reference on `ruint`.
#[allow(dead_code)]
pub mod sw {
    use ruint::aliases::U256;

    use super::Limbs;

    fn u(limbs: &Limbs) -> U256 {
        U256::from_limbs(*limbs)
    }

    pub fn mul(a: &Limbs, b: &Limbs) -> Limbs {
        u(a).wrapping_mul(u(b)).into_limbs()
    }

    pub fn div(a: &Limbs, b: &Limbs) -> Limbs {
        u(a).checked_div(u(b)).unwrap_or_default().into_limbs()
    }

    pub fn rem(a: &Limbs, b: &Limbs) -> Limbs {
        u(a).checked_rem(u(b)).unwrap_or_default().into_limbs()
    }

    pub fn add_mod(a: &Limbs, b: &Limbs, n: &Limbs) -> Limbs {
        u(a).add_mod(u(b), u(n)).into_limbs()
    }

    pub fn mul_mod(a: &Limbs, b: &Limbs, n: &Limbs) -> Limbs {
        u(a).mul_mod(u(b), u(n)).into_limbs()
    }

    pub fn pow(base: &Limbs, exponent: &Limbs) -> Limbs {
        u(base).wrapping_pow(u(exponent)).into_limbs()
    }
}

/// `base ^ exponent mod 2^256` by left-to-right square-and-multiply on a zkVM's wrapping
/// multiplication.
#[allow(dead_code)]
pub fn pow_with(mul: impl Fn(&Limbs, &Limbs) -> Limbs, base: &Limbs, exponent: &Limbs) -> Limbs {
    let Some(top) = (0..4).rev().find(|&i| exponent[i] != 0) else {
        return [1, 0, 0, 0];
    };
    let bits = 64 * top + 64 - exponent[top].leading_zeros() as usize;
    let mut result = *base;
    for i in (0..bits - 1).rev() {
        result = mul(&result, &result);
        if exponent[i / 64] >> (i % 64) & 1 == 1 {
            result = mul(&result, base);
        }
    }
    result
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_mul(a: *const Limbs, b: *const Limbs, result: *mut Limbs) -> i32 {
    unsafe { *result = ops::mul(&*a, &*b) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_div(a: *const Limbs, b: *const Limbs, result: *mut Limbs) -> i32 {
    unsafe { *result = ops::div(&*a, &*b) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_mod(a: *const Limbs, b: *const Limbs, result: *mut Limbs) -> i32 {
    unsafe { *result = ops::rem(&*a, &*b) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_addmod(
    a: *const Limbs,
    b: *const Limbs,
    n: *const Limbs,
    result: *mut Limbs,
) -> i32 {
    unsafe { *result = ops::add_mod(&*a, &*b, &*n) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_mulmod(
    a: *const Limbs,
    b: *const Limbs,
    n: *const Limbs,
    result: *mut Limbs,
) -> i32 {
    unsafe { *result = ops::mul_mod(&*a, &*b, &*n) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_u256_exp(
    base: *const Limbs,
    exponent: *const Limbs,
    result: *mut Limbs,
) -> i32 {
    unsafe { *result = ops::pow(&*base, &*exponent) };
    0
}
