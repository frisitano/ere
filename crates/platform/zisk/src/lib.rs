#![no_std]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod platform;
#[cfg(all(feature = "scoped-heap", target_os = "zkvm", target_vendor = "zisk"))]
mod scoped_heap;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod zkvm_io;

pub use ere_platform_core::Platform;
pub use ziskos;

pub use crate::platform::ZiskPlatform;
