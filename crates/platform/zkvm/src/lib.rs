//! Platform for guest programs built for the generic `riscv64ima-unknown-zkvm-elf` target.
//!
//! A guest built this way names no zkVM (see `ere-compiler-sdk`). It is linked against a zkVM SDK
//! (`libzkvm.a` and `zkvm.ld`), which provides `_start`, the [zkvm-standards] IO and accelerator
//! symbols, `abort`, and the `sys_*` functions `std` calls on `target_os = "zkvm"` (heap, stdio,
//! randomness).
//!
//! [zkvm-standards]: https://github.com/eth-act/zkvm-standards

use std::{boxed::Box, panic};

pub use ere_platform_core::Platform;

unsafe extern "C" {
    /// Abnormal termination, provided by the zkVM SDK.
    fn abort() -> !;
}

/// [`Platform`] backed by the zkVM SDK: input and output use the default `read_input` and
/// `write_output` symbols, and messages go to `std`'s stdout.
#[derive(Debug)]
pub struct ZkvmPlatform;

impl Platform for ZkvmPlatform {
    fn print(message: &str) {
        print!("{message}");
    }
}

// A validation guest must be deterministic, so randomness is never available.
#[cfg(target_os = "zkvm")]
getrandom::register_custom_getrandom!(no_randomness);

#[cfg(target_os = "zkvm")]
fn no_randomness(_: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}

/// Runs `guest` as the program's `main`: returns `0` when it returns, and terminates through the
/// SDK's `abort` when it panics.
///
/// `std` on `target_os = "zkvm"` would otherwise end a panic with a trapping instruction, which
/// zkVMs do not agree on.
pub fn run(guest: impl FnOnce()) -> i32 {
    panic::set_hook(Box::new(|info| {
        eprintln!("{info}");
        unsafe { abort() }
    }));
    guest();
    0
}
