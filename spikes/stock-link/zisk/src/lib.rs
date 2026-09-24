//! ZisK vendor library: everything comes from `ziskos`, built for a target with `os = "zkvm"` and
//! `vendor = "zisk"` so it selects its guest paths (`cfg(zisk_guest)`).
//!
//! - `_start` calls `int main(void)` and passes its return value to the ZisK exit syscall.
//! - `read_input`, `write_output` and every `zkvm_*` symbol are `ziskos` exports.
//!
//! `ziskos` leaves the panic handler to the program. This one is internal to the archive, because
//! `vendor-archive.sh` internalizes it, so it only catches panics raised inside `ziskos`.

#![no_std]

use ziskos as _;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    // ZisK exit syscall (93) with exit code 1, as `_start` uses for a non-zero `main` return.
    unsafe { core::arch::asm!("li a7, 93", "li a0, 1", "ecall", options(noreturn)) }
}
