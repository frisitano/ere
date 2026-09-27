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

/// `zkvm_u256_*`: multiplication, modular multiplication and exponentiation on SP1's
/// `UINT256_MUL` precompile, which computes `x * y mod m` with a zero `m` meaning 2^256; the rest
/// in software.
mod u256_ops {
    pub use super::u256::sw::{add_mod, div, rem};
    use super::u256::{Limbs, pow_with};

    unsafe extern "C" {
        // `x = x * y mod m`, with `m` stored right after `y`; both pointers 8-byte aligned.
        fn syscall_uint256_mulmod(x: *mut Limbs, y: *const Limbs);
    }

    fn uint256_mul(a: &Limbs, b: &Limbs, m: &Limbs) -> Limbs {
        let mut x = *a;
        let y_and_m = [*b, *m];
        unsafe { syscall_uint256_mulmod(&mut x, y_and_m.as_ptr()) };
        x
    }

    pub fn mul(a: &Limbs, b: &Limbs) -> Limbs {
        uint256_mul(a, b, &[0; 4])
    }

    pub fn mul_mod(a: &Limbs, b: &Limbs, n: &Limbs) -> Limbs {
        if *n == [0; 4] {
            [0; 4]
        } else {
            uint256_mul(a, b, n)
        }
    }

    pub fn pow(base: &Limbs, exponent: &Limbs) -> Limbs {
        pow_with(mul, base, exponent)
    }
}
