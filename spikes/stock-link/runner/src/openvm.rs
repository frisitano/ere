//! OpenVM interpreter execution, with the VM configuration and transpiler `ere-prover-openvm` uses.

use ere_verifier_openvm::NUM_PUBLIC_VALUES_BYTES;
use openvm_circuit::{
    arch::{ExecutionOutcome, VmExecutor, instructions::exe::VmExe},
    system::memory::merkle::public_values::extract_public_values,
};
use openvm_sdk::{F, StdIn};
use openvm_sdk_config::{SdkVmConfig, TranspilerConfig};
use openvm_transpiler::{FromElf, elf::Elf, openvm_platform::memory::MEM_SIZE};

use crate::Execution;

/// Instructions run per interpreter call before the state is checked for termination.
const CHUNK: u64 = 4096;

pub(crate) fn execute(elf: &[u8], input: &[u8]) -> Execution {
    let mut config = SdkVmConfig::standard();
    config.system.config = config.system.config.with_public_values_bytes(NUM_PUBLIC_VALUES_BYTES);
    let config = config.optimize();

    let elf = Elf::decode(elf, MEM_SIZE.try_into().unwrap()).expect("decode ELF");
    let exe = VmExe::<F>::from_elf(elf, config.transpiler()).expect("transpile ELF");

    let executor = VmExecutor::new(config).expect("VM executor");
    let instance = executor.interpreter_instance(&exe).expect("interpreter instance");

    let mut stdin = StdIn::default();
    stdin.write_bytes(input);

    // Run in chunks, then replay the last chunk one instruction at a time for an exact count.
    let mut instructions = 0;
    let mut last = None;
    let mut outcome = instance.execute_for(stdin.clone(), CHUNK).expect("execute");
    while let ExecutionOutcome::Suspended(state) = outcome {
        instructions += CHUNK;
        last = Some(state.clone());
        outcome = instance.execute_from_state_for(state, CHUNK).expect("execute");
    }
    let step = |state| instance.execute_from_state_for(state, 1).expect("execute");
    let mut outcome = match last {
        Some(state) => step(state),
        None => instance.execute_for(stdin, 1).expect("execute"),
    };
    instructions += 1;
    let state = loop {
        match outcome {
            ExecutionOutcome::Suspended(state) => outcome = step(state),
            ExecutionOutcome::Terminated(state) => break state,
        }
        instructions += 1;
    };

    Execution {
        output: extract_public_values(NUM_PUBLIC_VALUES_BYTES, &state.memory.memory),
        instructions,
    }
}
