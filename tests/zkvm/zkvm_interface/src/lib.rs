//! zkVM-agnostic guest: names no zkVM and is linked against a zkVM SDK.

use ere_platform_zkvm::{ZkvmPlatform, run};
use ere_util_test::program::{Program, zkvm_interface::ZkvmInterfaceProgram};

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    run(ZkvmInterfaceProgram::run::<ZkvmPlatform>)
}
