//! The [zkvm-standards] I/O interface's `abort` (`zkvm_io.h`), which `ziskos` does not export.
//! `ziskos` exports `read_input` and `write_output` itself (or, with the `scoped-heap` feature,
//! [`crate::scoped_heap`] does), and [`crate::ZiskPlatform`]'s
//! [`Platform`](ere_platform_core::Platform) methods call all three through their defaults.
//!
//! [zkvm-standards]: https://github.com/eth-act/zkvm-standards

/// Failed termination: ZisK's exit syscall, as `ziskos`'s `_start` uses it for `main`'s return
/// value, with exit code 1.
#[unsafe(no_mangle)]
pub(crate) extern "C" fn abort() -> ! {
    unsafe { core::arch::asm!("ecall", in("a7") 93, in("a0") 1, options(noreturn)) }
}
