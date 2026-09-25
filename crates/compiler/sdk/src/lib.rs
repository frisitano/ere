#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod error;
mod rust_rv64ima;

pub use ere_compiler_core::*;

pub use crate::{error::Error, rust_rv64ima::SdkRustRv64ima};
