//! ZisK emulator execution, the same way `ere-prover-zisk` executes a guest.

use zisk_transpiler_riscv::Riscv2zisk;
use ziskemu::{Emu, EmuOptions};

use crate::Execution;

pub(crate) fn execute(elf: &[u8], input: &[u8]) -> Execution {
    let rom = Riscv2zisk::new(elf).run().expect("transpile ELF to ZisK ROM");

    // ZisK input is the length as a little-endian u64, then the bytes, padded to 8 bytes.
    let mut stdin = (input.len() as u64).to_le_bytes().to_vec();
    stdin.extend_from_slice(input);
    stdin.resize(stdin.len().next_multiple_of(8), 0);

    let mut emu = Emu::new(&rom);
    emu.ctx = emu.create_emu_context(stdin, &EmuOptions::default());
    emu.run_fast(&EmuOptions::default());
    assert!(emu.ctx.inst_ctx.end, "emulator did not terminate");
    assert!(!emu.ctx.inst_ctx.error, "emulator reported an error");

    Execution {
        output: emu.get_output_8(),
        instructions: emu.ctx.inst_ctx.step,
    }
}
