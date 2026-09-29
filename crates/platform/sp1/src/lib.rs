#![no_std]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

extern crate alloc;

use sp1_libzkevm as _;

#[cfg(all(feature = "c-abi", target_os = "zkvm"))]
mod c_abi;
mod platform;

pub use ere_platform_core::Platform;
pub use sp1_zkvm;

pub use crate::platform::SP1Platform;
