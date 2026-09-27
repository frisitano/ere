//! ZisK SDK library. Everything comes from `ziskos`, built for a target with `os = "zkvm"` and
//! `vendor = "zisk"` so it selects its guest paths (`cfg(zisk_guest)`):
//!
//! - `_start` calls `int main(void)` and passes its return value to the ZisK exit syscall.
//! - `read_input`, `write_output`, every `zkvm_*` symbol but `zkvm_u256_*`, the heap
//!   (`sys_alloc_aligned`) and most of the `sys_*` functions are `ziskos` exports.
//!
//! The termination functions and `sys_read`, which `ziskos` does not export, are defined here, and
//! `zkvm_u256_*` in `../shims/u256.rs` on `ziskos`'s `uint256` library.

#![no_std]

use ziskos as _;

unsafe extern "C" {
    fn sys_write(fd: u32, write_buf: *const u8, nbytes: usize);
}

#[path = "../../shims/u256.rs"]
mod u256;

/// `zkvm_u256_*` on ZisK's `arith256`/`arith256_mod` precompiles, through `ziskos`'s `uint256`
/// library, whose values are the same little-endian limbs. Its division panics on a zero divisor,
/// where the EVM gives zero.
mod u256_ops {
    use ziskos::zisklib::{
        add_mod256, mul_mod256, wrapping_div256, wrapping_mul256, wrapping_pow256, wrapping_rem256,
    };

    use super::u256::Limbs;

    pub fn mul(a: &Limbs, b: &Limbs) -> Limbs {
        wrapping_mul256(a, b)
    }

    pub fn div(a: &Limbs, b: &Limbs) -> Limbs {
        if *b == [0; 4] {
            [0; 4]
        } else {
            wrapping_div256(a, b)
        }
    }

    pub fn rem(a: &Limbs, b: &Limbs) -> Limbs {
        if *b == [0; 4] {
            [0; 4]
        } else {
            wrapping_rem256(a, b)
        }
    }

    pub fn add_mod(a: &Limbs, b: &Limbs, n: &Limbs) -> Limbs {
        add_mod256(a, b, n)
    }

    pub fn mul_mod(a: &Limbs, b: &Limbs, n: &Limbs) -> Limbs {
        mul_mod256(a, b, n)
    }

    pub fn pow(base: &Limbs, exponent: &Limbs) -> Limbs {
        wrapping_pow256(base, exponent)
    }
}

/// ZisK's exit syscall, as `_start` uses it for `main`'s return value.
#[unsafe(no_mangle)]
extern "C" fn exit(code: i32) -> ! {
    unsafe { core::arch::asm!("ecall", in("a7") 93, in("a0") code, options(noreturn)) }
}

#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    exit(1)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sys_panic(msg_ptr: *const u8, len: usize) -> ! {
    unsafe { sys_write(2, msg_ptr, len) };
    exit(1)
}

/// There is no standard input.
#[unsafe(no_mangle)]
unsafe extern "C" fn sys_read(_fd: u32, _recv_buf: *mut u8, _nrequested: usize) -> usize {
    0
}

// Panics raised inside `ziskos` itself. `vendor-archive.sh` keeps this handler internal.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    abort()
}
