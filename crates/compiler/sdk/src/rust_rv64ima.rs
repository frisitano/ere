use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use ere_compiler_core::{Compiler, Elf};
use ere_util_compile::{CargoBuildCmd, CommonError, RustTarget, parse_cargo_build_options};
use tempfile::tempdir;

use crate::Error;

/// Generic zkVM guest target: the stock `riscv64ima` spec with `os = "zkvm"`, so `std` builds on
/// upstream's zkVM platform port. It names no zkVM.
const TARGET: RustTarget = RustTarget::SpecJson {
    name: "riscv64ima-unknown-zkvm-elf",
    json: include_str!("./rust_rv64ima/riscv64ima-unknown-zkvm-elf.json"),
};

const RUSTFLAGS: &[&str] = &[
    // Emit LLVM bitcode, so the link optimizes guest and SDK code as one module.
    "-C",
    "linker-plugin-lto",
    "-C",
    "passes=lower-atomic",
    "--cfg",
    "getrandom_backend=\"custom\"",
];

const CARGO_BUILD_OPTIONS: &[&str] = &[
    "-Zbuild-std=std,panic_abort",
    "-Zbuild-std-features=compiler-builtins-mem",
    "-Zjson-target-spec",
    // One fat-LTO bitcode module exporting only `main`, whatever the guest's own profile says.
    "--config",
    "profile.release.lto=\"fat\"",
    "--config",
    "profile.release.codegen-units=1",
    "--config",
    "profile.release.panic=\"abort\"",
];

/// Compiler for a zkVM-agnostic Rust guest, linked against a zkVM SDK.
///
/// The guest package's library target must be a `staticlib` defining `int main(void)`. It is built
/// with a stock nightly toolchain into a guest object (`lib<name>.a`: one LLVM bitcode module plus
/// native `compiler_builtins`) whose only undefined symbols are the zkVM guest ABI. It is linked
/// by ere's `sdk/link.sh`, which checks it against the ABI, adds the ISA extensions the SDK lists
/// in `zkvm.features`, and runs `ld.lld -T <sdk>/zkvm.ld -L <sdk>` with full LTO across the guest
/// and the SDK, so accelerator calls can inline.
///
/// `ld.lld` is taken from `ERE_LD_LLD`, or from `PATH`, and the LLVM tools from `ERE_LLVM_BIN`.
/// Their LLVM must be at least as new as the toolchain's, because bitcode is only read forward.
pub struct SdkRustRv64ima {
    sdk: PathBuf,
}

impl SdkRustRv64ima {
    /// Links against the SDK in directory `sdk`.
    pub fn new(sdk: impl Into<PathBuf>) -> Self {
        Self { sdk: sdk.into() }
    }

    /// Builds the guest object of the package in `guest_directory` and returns its path.
    pub fn build_guest_object(
        guest_directory: impl AsRef<Path>,
        args: &[String],
    ) -> Result<PathBuf, Error> {
        let toolchain = env::var("ERE_RUST_TOOLCHAIN").unwrap_or_else(|_| "nightly".into());
        let options = parse_cargo_build_options(args)?;
        Ok(CargoBuildCmd::new()
            .toolchain(toolchain)
            .build_options(CARGO_BUILD_OPTIONS)
            .rustflags(RUSTFLAGS)
            .features(&options.features)
            .ignore_rust_version(options.ignore_rust_version)
            .exec_staticlib(guest_directory, TARGET)?)
    }

    /// Links the guest object at `guest_object` against the SDK and returns the ELF.
    pub fn link(&self, guest_object: impl AsRef<Path>) -> Result<Elf, Error> {
        let tempdir = tempdir().map_err(CommonError::tempdir)?;
        let elf_path = tempdir.path().join("guest.elf");
        // `sdk/link.sh` in this repository: the one link command, shared with CI.
        let link = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sdk/link.sh");
        let mut cmd = Command::new(link);
        cmd.arg(&self.sdk).arg(guest_object.as_ref()).arg(&elf_path);
        if let Ok(ld) = env::var("ERE_LD_LLD") {
            cmd.env("LD_LLD", ld);
        }
        if let Ok(llvm_bin) = env::var("ERE_LLVM_BIN") {
            cmd.env("LLVM_BIN", llvm_bin);
        }
        let output = cmd
            .output()
            .map_err(|err| CommonError::command(&cmd, err))?;
        if !output.status.success() {
            Err(CommonError::command_exit_non_zero(
                &cmd,
                output.status,
                Some(&output),
            ))?;
        }
        let elf = std::fs::read(&elf_path)
            .map_err(|err| CommonError::read_file("elf", &elf_path, err))?;
        Ok(Elf(elf))
    }
}

impl Compiler for SdkRustRv64ima {
    type Error = Error;

    fn compile(
        &self,
        guest_directory: impl AsRef<Path>,
        args: &[String],
    ) -> Result<Elf, Self::Error> {
        self.link(Self::build_guest_object(guest_directory, args)?)
    }
}
