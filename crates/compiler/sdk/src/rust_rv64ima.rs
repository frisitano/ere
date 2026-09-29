use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use ere_compiler_core::{Compiler, Elf};
use ere_util_compile::{CargoBuildCmd, CommonError, RustTarget, parse_cargo_build_options};
use tempfile::tempdir;

use crate::Error;

/// How a guest object is built for one guest target.
struct GuestTarget {
    target: RustTarget,
    /// Default toolchain, overridden by `ERE_RUST_TOOLCHAIN`.
    toolchain: &'static str,
    /// Set for `cargo build`, e.g. to let a stable toolchain take `-Z` options.
    envs: &'static [(&'static str, &'static str)],
    rustflags: &'static [&'static str],
    build_std: &'static str,
}

/// Bare-metal `no_std` guest target: the stock `riscv64im-unknown-none-elf` spec with
/// compare-and-swap, which `alloc::sync` needs. A zkVM guest runs on one thread, so its atomics are
/// lowered to plain loads and stores (`-Cpasses=lower-atomic`) before the bitcode reaches the link.
/// It names no zkVM.
const RV64IM: GuestTarget = GuestTarget {
    target: RustTarget::SpecJson {
        name: "riscv64im-unknown-none-elf",
        json: include_str!("./rust_rv64ima/riscv64im-unknown-none-elf.json"),
    },
    // A stable toolchain (LLVM 22.1.2). A guest object must come from an LLVM no newer than the
    // linker's. `RUSTC_BOOTSTRAP` admits the two `-Z` options a Tier 3 target needs.
    toolchain: "1.95.0",
    envs: &[("RUSTC_BOOTSTRAP", "1")],
    rustflags: &["-C", "passes=lower-atomic"],
    build_std: "-Zbuild-std=core,alloc",
};

/// Generic `std` guest target: the stock `riscv64ima` spec with `os = "zkvm"`, so `std` builds on
/// upstream's zkVM platform port. It names no zkVM.
const RV64IMA_ZKVM: GuestTarget = GuestTarget {
    target: RustTarget::SpecJson {
        name: "riscv64ima-unknown-zkvm-elf",
        json: include_str!("./rust_rv64ima/riscv64ima-unknown-zkvm-elf.json"),
    },
    // The nightly `sdk/build.sh` builds SDKs with (LLVM 22.1.0).
    toolchain: "nightly-2026-03-17",
    envs: &[],
    rustflags: &[
        "-C",
        "passes=lower-atomic",
        "--cfg",
        "getrandom_backend=\"custom\"",
    ],
    build_std: "-Zbuild-std=std,panic_abort",
};

impl GuestTarget {
    fn build_guest_object(
        &self,
        guest_directory: impl AsRef<Path>,
        args: &[String],
    ) -> Result<PathBuf, Error> {
        let toolchain = env::var("ERE_RUST_TOOLCHAIN").unwrap_or_else(|_| self.toolchain.into());
        let options = parse_cargo_build_options(args)?;
        let build_options = [
            self.build_std,
            "-Zbuild-std-features=compiler-builtins-mem",
            "-Zjson-target-spec",
            // One fat-LTO bitcode module exporting only `main`, whatever the guest's own profile
            // says.
            "--config",
            "profile.release.lto=\"fat\"",
            "--config",
            "profile.release.codegen-units=1",
            "--config",
            "profile.release.panic=\"abort\"",
        ];
        // Emit LLVM bitcode, so the link optimizes guest and SDK code as one module.
        let rustflags = [&["-C", "linker-plugin-lto"], self.rustflags].concat();
        let mut cmd = CargoBuildCmd::new()
            .toolchain(toolchain)
            .build_options(&build_options)
            .rustflags(&rustflags)
            .features(&options.features)
            .ignore_rust_version(options.ignore_rust_version);
        for (key, value) in self.envs {
            cmd = cmd.env(key, value);
        }
        Ok(cmd.exec_staticlib(guest_directory, self.target)?)
    }
}

/// Compiler for a zkVM-agnostic `no_std` Rust guest, linked against a zkVM SDK.
///
/// The guest package's library target must be a `staticlib` defining `int main(void)`, and use
/// `ere-platform-zkvm` as its runtime (allocator and panic handler). It is built with a stable
/// toolchain for the bare-metal `riscv64im` target into a guest object (`lib<name>.a`: one LLVM
/// bitcode module plus native `compiler_builtins`) whose only undefined symbols are the zkVM guest
/// ABI, and linked by ere's `sdk/link.sh`, which checks it against the ABI, adds the ISA extensions
/// the SDK lists in `zkvm.features`, and runs `ld.lld -T <sdk>/zkvm.ld -L <sdk>` with full LTO
/// across the guest and the SDK, so accelerator calls can inline.
///
/// `ld.lld` is taken from `ERE_LD_LLD`, or from `PATH`, and the LLVM tools from `ERE_LLVM_BIN`.
/// Their LLVM must be at least as new as the toolchain's, because bitcode is only read forward.
pub struct SdkRustRv64im {
    sdk: PathBuf,
}

impl SdkRustRv64im {
    /// Links against the SDK in directory `sdk`.
    pub fn new(sdk: impl Into<PathBuf>) -> Self {
        Self { sdk: sdk.into() }
    }

    /// Builds the guest object of the package in `guest_directory` and returns its path.
    pub fn build_guest_object(
        guest_directory: impl AsRef<Path>,
        args: &[String],
    ) -> Result<PathBuf, Error> {
        RV64IM.build_guest_object(guest_directory, args)
    }

    /// Links the guest object at `guest_object` against the SDK and returns the ELF.
    pub fn link(&self, guest_object: impl AsRef<Path>) -> Result<Elf, Error> {
        link(&self.sdk, guest_object.as_ref())
    }
}

impl Compiler for SdkRustRv64im {
    type Error = Error;

    fn compile(
        &self,
        guest_directory: impl AsRef<Path>,
        args: &[String],
    ) -> Result<Elf, Self::Error> {
        self.link(Self::build_guest_object(guest_directory, args)?)
    }
}

/// Compiler for a zkVM-agnostic Rust guest that uses `std`, linked against a zkVM SDK.
///
/// As [`SdkRustRv64im`], but built with the pinned nightly for the `riscv64ima` target with
/// `target_os = "zkvm"`, where `std` calls the SDK's `sys_*` functions.
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
        RV64IMA_ZKVM.build_guest_object(guest_directory, args)
    }

    /// Links the guest object at `guest_object` against the SDK and returns the ELF.
    pub fn link(&self, guest_object: impl AsRef<Path>) -> Result<Elf, Error> {
        link(&self.sdk, guest_object.as_ref())
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

/// Links `guest_object` against the SDK in `sdk` with `sdk/link.sh` in this repository, the one
/// link command, shared with CI.
fn link(sdk: &Path, guest_object: &Path) -> Result<Elf, Error> {
    let tempdir = tempdir().map_err(CommonError::tempdir)?;
    let elf_path = tempdir.path().join("guest.elf");
    let link = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sdk/link.sh");
    let mut cmd = Command::new(link);
    cmd.arg(sdk).arg(guest_object).arg(&elf_path);
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
    let elf =
        std::fs::read(&elf_path).map_err(|err| CommonError::read_file("elf", &elf_path, err))?;
    Ok(Elf(elf))
}
