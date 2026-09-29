//! ZisK SDK library. Everything comes from `ziskos`, built for a target with `os = "zkvm"` and
//! `vendor = "zisk"` so it selects its guest paths (`cfg(zisk_guest)`):
//!
//! - `_start` calls `int main(void)` and passes its return value to the ZisK exit syscall.
//! - `read_input`, `write_output`, every `zkvm_*` symbol and the heap (`sys_alloc_aligned`) are
//!   `ziskos` exports.
//!
//! `abort`, which `ziskos` does not export, is defined here.

#![no_std]

use ziskos as _;

/// Failed termination: ZisK's exit syscall, as `_start` uses it for `main`'s return value, with
/// exit code 1.
#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    unsafe { core::arch::asm!("ecall", in("a7") 93, in("a0") 1, options(noreturn)) }
}

// Panics raised inside `ziskos` itself. `vendor-archive.sh` keeps this handler internal.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    abort()
}
