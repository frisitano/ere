//! SP1 minimal-executor execution, the same way `ere-prover-sp1` executes a guest.

use std::sync::Arc;

use sp1_core_executor::{MinimalExecutorEnum, Program};

use crate::Execution;

pub(crate) fn execute(elf: &[u8], input: &[u8]) -> Execution {
    let program = Arc::new(Program::from(elf).expect("disassemble ELF"));
    let mut executor = MinimalExecutorEnum::new(program, false, None);

    // `libzkevm`'s `read_input` returns the first hint-stream chunk, so the input is one chunk.
    executor.with_input(input);
    while !executor.is_done() {
        executor.execute_chunk();
    }
    assert_eq!(executor.exit_code(), 0, "guest exited with a failure code");

    Execution {
        output: executor.public_values_stream().clone(),
        instructions: executor.global_clk(),
    }
}
