#![no_std]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

extern crate alloc;

#[cfg(all(feature = "c-abi", target_os = "openvm"))]
mod c_abi;
mod platform;
#[cfg(feature = "zkvm-accelerator")]
mod zkvm_accelerator;

pub use ere_platform_core::Platform;
pub use openvm;

pub use crate::platform::OpenVMPlatform;
