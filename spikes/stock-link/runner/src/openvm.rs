//! OpenVM interpreter execution, with the VM configuration and transpiler `ere-prover-openvm` uses.

use ere_verifier_openvm::NUM_PUBLIC_VALUES_BYTES;
use openvm_circuit::{
    arch::{ExecutionOutcome, VmExecutor, VmState, instructions::exe::VmExe},
    system::memory::merkle::public_values::extract_public_values,
};
use openvm_sdk::{F, StdIn};
use openvm_sdk_config::{SdkVmConfig, TranspilerConfig};
use openvm_transpiler::{FromElf, elf::Elf, openvm_platform::memory::MEM_SIZE};

use crate::Execution;

/// Instructions run per interpreter call before the state is checked for termination. Each chunk
/// clones the VM state once, so this is large.
const CHUNK: u64 = 1 << 22;

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

    // Run in chunks from a saved state; once a chunk terminates, binary-search the exact number of
    // instructions it ran by replaying it from the saved state.
    let mut instructions = 0;
    let mut saved = VmState::initial(
        executor.config.as_ref(),
        &exe.init_memory,
        exe.pc_start,
        stdin,
    );
    let state = loop {
        match instance.execute_from_state_for(saved.clone(), CHUNK).expect("execute") {
            ExecutionOutcome::Suspended(state) => {
                instructions += CHUNK;
                saved = state;
            }
            ExecutionOutcome::Terminated(state) => {
                // Smallest `n` for which the chunk terminates within `n` instructions.
                let (mut low, mut high) = (1, CHUNK);
                while low < high {
                    let mid = low + (high - low) / 2;
                    match instance.execute_from_state_for(saved.clone(), mid).expect("execute") {
                        ExecutionOutcome::Terminated(_) => high = mid,
                        ExecutionOutcome::Suspended(_) => low = mid + 1,
                    }
                }
                instructions += low;
                break state;
            }
        }
    };

    Execution {
        output: extract_public_values(NUM_PUBLIC_VALUES_BYTES, &state.memory.memory),
        instructions,
    }
}
