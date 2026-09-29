//! zkVM-agnostic `no_std` guest: names no zkVM and is linked against a zkVM SDK.

#![no_std]

use ere_platform_zkvm::{ZkvmPlatform, run};
use ere_util_test::program::{Program, zkvm_interface::ZkvmInterfaceProgram};

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    run(ZkvmInterfaceProgram::run::<ZkvmPlatform>)
}
