use ere_platform_core::Platform;

/// Maximum bytes of output the guest may reveal.
#[cfg_attr(not(target_os = "openvm"), allow(dead_code))]
pub const MAX_OUTPUT_BYTES: usize = 256;

/// OpenVM [`Platform`] implementation.
///
/// Note that the maximum output size is 256 bytes, and output less than 256
/// bytes will be padded to 256 bytes.
pub struct OpenVMPlatform;

/// `read_input`, `write_output` and `abort` are the trait's defaults, on the zkvm-standards C
/// functions that `zkvm_io.rs` exports.
impl Platform for OpenVMPlatform {
    fn print(message: &str) {
        openvm::io::print(message)
    }
}
