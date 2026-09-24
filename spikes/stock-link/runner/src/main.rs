//! Executes a stock-linked guest ELF on one zkVM and checks its output.
//!
//! The guest outputs `keccak256(input) || sha256(input)`. The runner compares that with a host
//! computation and reports how many guest instructions ran, which separates an accelerated build
//! from a software fallback.
//!
//! Usage: stock-link-runner <elf> <input-len>

use std::{env, fs};

use sha2::Digest;

#[cfg(feature = "openvm")]
mod openvm;
#[cfg(feature = "zisk")]
mod zisk;

/// Result of one execution.
struct Execution {
    output: Vec<u8>,
    instructions: u64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let elf = fs::read(&args[1]).expect("read ELF");
    let len: usize = args[2].parse().expect("input length");
    let input: Vec<u8> = (0..len).map(|i| (i * 31 + 7) as u8).collect();

    let mut expected = sha3::Keccak256::digest(&input).to_vec();
    expected.extend(sha2::Sha256::digest(&input));

    #[cfg(feature = "openvm")]
    let (zkvm, execution) = ("openvm", openvm::execute(&elf, &input));
    #[cfg(feature = "zisk")]
    let (zkvm, execution) = ("zisk", zisk::execute(&elf, &input));

    let output = &execution.output[..64];
    println!("zkvm          {zkvm}");
    println!("input         {len} bytes");
    println!("instructions  {}", execution.instructions);
    println!("output        {}", hex::encode(output));
    println!("expected      {}", hex::encode(&expected));
    assert_eq!(output, expected.as_slice(), "guest output does not match the host");
    println!("match         true");
}
