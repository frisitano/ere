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

#[path = "../../shims/u256.rs"]
mod u256;

/// `zkvm_u256_*`: multiplication, modular multiplication and exponentiation on SP1's 256-bit
/// precompiles; the rest in software.
///
/// - `mul` uses `UINT256_MUL_CARRY` (`a * b + c` into separate low and high words), which takes
///   every operand by pointer, so the caller's operands are read where they are, with no copy.
/// - `mul_mod` uses `UINT256_MUL` (`x = x * y mod m` in place, with `y` and `m` adjacent), so `m`
///   and one operand are copied into its buffer; the product lands in `result`.
mod u256_ops {
    use core::{mem::MaybeUninit, ptr};

    pub use super::u256::sw::{add_mod, div, rem};
    use super::u256::{Limbs, pow_with};

    unsafe extern "C" {
        // `x = x * y mod m`, with `m` stored right after `y`; both pointers 8-byte aligned.
        fn syscall_uint256_mulmod(x: *mut Limbs, y: *const Limbs);
        // `d = low(a * b + c)`, `e = high(a * b + c)`; all pointers 8-byte aligned.
        fn syscall_uint256_mul_with_carry(
            a: *const Limbs,
            b: *const Limbs,
            c: *const Limbs,
            d: *mut Limbs,
            e: *mut Limbs,
        );
    }

    static ZERO: Limbs = [0; 4];

    /// # Safety
    /// As `zkvm_u256_mul`.
    pub unsafe fn mul_to(result: *mut Limbs, a: *const Limbs, b: *const Limbs) {
        let mut high = MaybeUninit::<Limbs>::uninit();
        unsafe {
            if ptr::eq(result.cast_const(), a) || ptr::eq(result.cast_const(), b) {
                // SP1 does not document an output overlapping an input, so write elsewhere.
                let mut low = MaybeUninit::<Limbs>::uninit();
                syscall_uint256_mul_with_carry(a, b, &ZERO, low.as_mut_ptr(), high.as_mut_ptr());
                *result = low.assume_init();
            } else {
                syscall_uint256_mul_with_carry(a, b, &ZERO, result, high.as_mut_ptr());
            }
        }
    }

    /// By value, for `pow`'s square-and-multiply on its own locals.
    pub fn mul(a: &Limbs, b: &Limbs) -> Limbs {
        let mut x = MaybeUninit::<Limbs>::uninit();
        unsafe {
            mul_to(x.as_mut_ptr(), a, b);
            x.assume_init()
        }
    }

    /// # Safety
    /// As `zkvm_u256_mulmod`.
    pub unsafe fn mul_mod_to(
        result: *mut Limbs,
        a: *const Limbs,
        b: *const Limbs,
        n: *const Limbs,
    ) {
        unsafe {
            if *n == [0; 4] {
                *result = [0; 4];
                return;
            }
            // `y ‖ m` first: `result` may alias `b` or `n`. Then `a` into `result` unless it is
            // `a`.
            let mut y_and_m = MaybeUninit::<[Limbs; 2]>::uninit();
            let y = y_and_m.as_mut_ptr().cast::<Limbs>();
            ptr::copy_nonoverlapping(b, y, 1);
            ptr::copy_nonoverlapping(n, y.add(1), 1);
            if !ptr::eq(result.cast_const(), a) {
                ptr::copy_nonoverlapping(a, result, 1);
            }
            syscall_uint256_mulmod(result, y.cast_const());
        }
    }

    pub fn mul_mod(a: &Limbs, b: &Limbs, n: &Limbs) -> Limbs {
        let mut x = MaybeUninit::<Limbs>::uninit();
        unsafe {
            mul_mod_to(x.as_mut_ptr(), a, b, n);
            x.assume_init()
        }
    }

    pub fn pow(base: &Limbs, exponent: &Limbs) -> Limbs {
        pow_with(mul, base, exponent)
    }
}
