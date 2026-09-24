#!/usr/bin/env python3
"""Linker wrapper that turns a linker-plugin-LTO link into one full-LTO link.

rustc always emits ThinLTO bitcode under `-Clinker-plugin-lto`, and lld's `--lto=full` only
accepts unified-LTO bitcode, which rustc cannot emit. This wrapper takes rustc's `rust-lld`
command line, merges every bitcode input into one module without a ThinLTO summary, and calls
`rust-lld` with that module in place of the bitcode inputs. lld then optimizes guest and vendor
code as a single module, as `lto = "fat"` does inside rustc.

- Guest objects and the vendor archives named in FULL_LTO_LIBS are linked in whole: the vendor
  module holds `_start`, which nothing in the guest references.
- Members of the other archives (rlibs such as `core`) are linked with `--only-needed`, which
  approximates the linker's lazy archive extraction.
- Native objects and native archive members pass through unchanged.

Usage: -Clinker=full-lto-link.py, with FULL_LTO_LIBS="zkvm_openvm_full ..." in the environment.
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

LLVM_BIN = Path(os.environ.get("LLVM_BIN", "/opt/homebrew/opt/llvm/bin"))
BITCODE_MAGIC = b"BC\xc0\xde"


def rust_lld() -> Path:
    sysroot = subprocess.run(
        ["rustc", "+nightly", "--print", "sysroot"], capture_output=True, text=True, check=True
    ).stdout.strip()
    return next(Path(sysroot, "lib", "rustlib").glob("*/bin/rust-lld"))


def is_bitcode(path: Path) -> bool:
    with open(path, "rb") as f:
        return f.read(4) == BITCODE_MAGIC


def is_elf(path: Path) -> bool:
    with open(path, "rb") as f:
        return f.read(4) == b"\x7fELF"


def extract(archive: Path, into: Path) -> list[Path]:
    into.mkdir(parents=True)
    subprocess.run([LLVM_BIN / "llvm-ar", "x", archive], cwd=into, check=True)
    return sorted(into.iterdir())


def main() -> None:
    args = sys.argv[1:]
    vendor_libs = set(os.environ.get("FULL_LTO_LIBS", "").split())
    search = [a[2:] for a in args if a.startswith("-L") and len(a) > 2]
    search += [args[i + 1] for i, a in enumerate(args[:-1]) if a == "-L"]

    work = Path(tempfile.mkdtemp(prefix="full-lto-", dir=os.environ.get("AGENT_TMPDIR")))
    whole, needed, passthrough = [], [], []

    for arg in args:
        if arg.startswith("-l") and arg[2:] in vendor_libs:
            path = next(Path(d, f"lib{arg[2:]}.a") for d in search if Path(d, f"lib{arg[2:]}.a").exists())
            members, target = extract(path, work / arg[2:]), whole
        elif arg.endswith((".rlib", ".a")) and Path(arg).is_file():
            members, target = extract(Path(arg), work / f"{len(needed)}-{Path(arg).name}"), needed
        elif arg.endswith(".o") and Path(arg).is_file() and is_bitcode(Path(arg)):
            whole.append(Path(arg))
            continue
        else:
            passthrough.append(arg)
            continue

        target += [m for m in members if is_bitcode(m)]
        native = [m for m in members if is_elf(m)]
        if native:
            repacked = work / f"native-{len(passthrough)}.a"
            subprocess.run([LLVM_BIN / "llvm-ar", "rcs", repacked, *native], check=True)
            passthrough.append(str(repacked))

    merged = work / "merged.bc"
    subprocess.run([LLVM_BIN / "llvm-link", *whole, "-o", work / "whole.bc"], check=True)
    subprocess.run(
        [LLVM_BIN / "llvm-link", work / "whole.bc", "--only-needed", *needed, "-o", merged],
        check=True,
    )

    lld = [rust_lld()]
    if passthrough[:2] != ["-flavor", "gnu"]:
        lld += ["-flavor", "gnu"]
    sys.exit(subprocess.run([*lld, *passthrough, merged]).returncode)


if __name__ == "__main__":
    main()
