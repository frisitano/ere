#![no_std]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

#[cfg(all(feature = "c-abi", target_os = "zkvm", target_vendor = "zisk"))]
mod c_abi;
mod platform;

pub use ere_platform_core::Platform;
pub use ziskos;

pub use crate::platform::ZiskPlatform;
