//! Platform for guest programs that name no zkVM and are linked against a zkVM SDK (`libzkvm.a` and
//! `zkvm.ld`), which provides `_start`, the [zkvm-standards] IO and accelerator symbols, `abort`
//! and `sys_alloc_aligned` (see `ere-compiler-sdk`).
//!
//! It has two modes, chosen by the guest's target:
//!
//! - **`no_std`** on the bare-metal `riscv64im` target (`target_os = "none"`). This crate is then
//!   the guest's runtime: its global allocator takes memory from the SDK's `sys_alloc_aligned`, and
//!   its panic handler ends the program through the SDK's `abort`. The guest needs nothing from the
//!   SDK beyond the standard symbols and those two.
//! - **`std`** on the `riscv64ima` target with `target_os = "zkvm"`, where upstream's zkVM port of
//!   `std` calls the SDK's `sys_*` functions and [`run`] routes panics to `abort`.
//!
//! The `no_std` allocator uses the SDK's heap, as `std` does on `target_os = "zkvm"`, rather than
//! the `_heap_start`/`_heap_end` region the zkvm-standards reserve for the application: zkVM
//! libraries allocate from that same region today.
//!
//! [zkvm-standards]: https://github.com/eth-act/zkvm-standards

#![cfg_attr(bare_metal, no_std)]

pub use ere_platform_core::Platform;

unsafe extern "C" {
    /// Abnormal termination, provided by the zkVM SDK.
    fn abort() -> !;
}

/// [`Platform`] backed by the zkVM SDK: input and output use the standard `read_input` and
/// `write_output` symbols.
#[derive(Debug)]
pub struct ZkvmPlatform;

#[cfg(not(bare_metal))]
impl Platform for ZkvmPlatform {
    fn print(message: &str) {
        std::print!("{message}");
    }
}

#[cfg(bare_metal)]
impl Platform for ZkvmPlatform {}

#[cfg(bare_metal)]
mod runtime {
    use core::alloc::{GlobalAlloc, Layout};

    unsafe extern "C" {
        /// The zkVM's heap: `bytes` bytes aligned to `align`, never freed.
        fn sys_alloc_aligned(bytes: usize, align: usize) -> *mut u8;
    }

    struct SdkHeap;

    unsafe impl GlobalAlloc for SdkHeap {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe { sys_alloc_aligned(layout.size(), layout.align()) }
        }

        // The SDK's heap does not free, as on `target_os = "zkvm"`.
        unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
    }

    #[global_allocator]
    static HEAP: SdkHeap = SdkHeap;

    #[panic_handler]
    fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
        unsafe { super::abort() }
    }
}

/// Runs `guest` as the program's `main`: returns `0` when it returns, and terminates through the
/// SDK's `abort` when it panics.
///
/// Under `std`, a panic would otherwise end with a trapping instruction, which zkVMs do not agree
/// on. Without `std`, this crate's panic handler already calls `abort`.
pub fn run(guest: impl FnOnce()) -> i32 {
    #[cfg(not(bare_metal))]
    std::panic::set_hook(std::boxed::Box::new(|info| {
        std::eprintln!("{info}");
        unsafe { abort() }
    }));
    guest();
    0
}

// A validation guest must be deterministic, so randomness is never available.
#[cfg(target_os = "zkvm")]
getrandom::register_custom_getrandom!(no_randomness);

#[cfg(target_os = "zkvm")]
fn no_randomness(_: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}
