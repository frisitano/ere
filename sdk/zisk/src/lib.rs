//! ZisK SDK library. Everything comes from `ziskos`, built for a target with `os = "zkvm"` and
//! `vendor = "zisk"` so it selects its guest paths (`cfg(zisk_guest)`):
//!
//! - `_start` calls `int main(void)` and passes its return value to the ZisK exit syscall.
//! - `read_input`, `write_output`, every `zkvm_*` symbol, the heap (`sys_alloc_aligned`) and most
//!   of the `sys_*` functions are `ziskos` exports.
//!
//! The termination functions and `sys_read`, which `ziskos` does not export, are defined here.

#![no_std]

use ziskos as _;

unsafe extern "C" {
    fn sys_write(fd: u32, write_buf: *const u8, nbytes: usize);
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
