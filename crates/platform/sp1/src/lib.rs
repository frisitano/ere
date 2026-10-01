#![no_std]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

extern crate alloc;

use sp1_libzkevm as _;

mod platform;
#[cfg(all(feature = "scoped-heap", target_os = "zkvm"))]
mod scoped_heap;

pub use ere_platform_core::Platform;
pub use sp1_zkvm;

pub use crate::platform::SP1Platform;
