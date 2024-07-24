//! Functions to generate result from zkVM given ELF and input.

use std::io::Write;

use anyhow::{Context, Result};
use clap::Parser;
use ethers::abi::Token;
use risc0_zkvm::{
    Executor, ExecutorEnv, LocalProver,
};
use alloy_sol_types::{sol, SolType};
use alloy::{
    primitives::{keccak256, Address, Signature},
    signers::Signer,
};
use risc0_binfmt::compute_image_id;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
enum Command {
    /// Prove the RISC-V ELF binary.
    Prove {
        /// The guest binary path
        guest_binary_path: String,

        /// The hex encoded input to provide to the guest binary
        input: String,

        /// Job ID (generated by JobManager contract)
        job_id: u32,

        /// The maximum number of cycles to run the program for
        max_cycles: u64,
    },
}

/// Run the CLI.
pub fn main() -> Result<()> {
    match Command::parse() {
        Command::Prove {
            guest_binary_path,
            input,
            job_id,
            max_cycles
        } => prove_ffi(
            guest_binary_path,
            hex::decode(input.strip_prefix("0x").unwrap_or(&input))?,
            job_id,
            max_cycles,
        )?,
    };

    Ok(())
}

/// Prints on stdio the Ethereum ABI and hex encoded proof.
fn prove_ffi(elf_path: String, input: Vec<u8>, job_id: u32, max_cycles: u64) -> Result<()> {
    let elf = std::fs::read(elf_path).unwrap();
    let image_id = compute_image_id(&elf)?;
    let image_id_bytes = image_id.as_bytes().try_into().expect("image id is 32 bytes");
    let journal = prove(&elf, &input, max_cycles)?;
    let result_with_metadata = abi_encode_result_with_metadata(job_id, input, max_cycles, image_id_bytes, journal);

    let calldata = vec![Token::Bytes(result_with_metadata)];
    let output = hex::encode(ethers::abi::encode(&calldata));

    // Forge test FFI calls expect hex encoded bytes sent to stdout
    print!("{output}");
    std::io::stdout()
        .flush()
        .context("failed to flush stdout buffer")?;
    Ok(())
}

/// Generates journal for the given elf and input.
fn prove(elf: &[u8], input: &[u8], max_cycles: u64) -> Result<Vec<u8>> {
    let env = ExecutorEnv::builder()
    .session_limit(Some(max_cycles))
    .write_slice(input)
    .build()?;

    let prover = LocalProver::new("locals only");
    let prove_info = prover.execute(env, elf)?;

    Ok(prove_info.journal.bytes)
}

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed the job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
pub type ResultWithMetadata = sol! {
    tuple(uint32,bytes32,uint64,bytes32,bytes)
};

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(job_id: u32, program_input: Vec<u8>, max_cycles: u64, program_verifying_key: &[u8; 32], raw_output: Vec<u8>) -> Vec<u8> {
    let program_input_hash = keccak256(program_input);
    ResultWithMetadata::abi_encode_params(&(
        job_id,
        program_input_hash,
        max_cycles,
        program_verifying_key,
        raw_output,
    ))
}

