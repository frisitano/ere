#![no_std]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

extern crate alloc;

mod platform;
#[cfg(feature = "zkvm-accelerator")]
mod zkvm_accelerator;
#[cfg(target_os = "openvm")]
mod zkvm_io;

pub use ere_platform_core::Platform;
pub use openvm;

pub use crate::platform::OpenVMPlatform;
