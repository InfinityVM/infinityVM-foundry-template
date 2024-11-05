//! Functions to execute a zkVM program on a given input.

use core::str::FromStr;
use std::io::Write;

use alloy::{
    primitives::{hex, keccak256, Address},
    signers::{local::LocalSigner, Signer},
};
use alloy_sol_types::{sol, SolType};
use anyhow::{Context, Result};
use clap::Parser;
use k256::ecdsa::SigningKey;
use risc0_binfmt::compute_image_id;
use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};

type K256LocalSigner = LocalSigner<SigningKey>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
enum Command {
    /// Execute the RISC-V ELF binary (returns the signed result)
    ExecuteOnchainJob {
        /// The guest binary path
        guest_binary_path: String,

        /// The hex-encoded input to provide to the guest binary
        onchain_input: String,

        /// The hex-encoded Job ID
        job_id: String,

        /// The maximum number of cycles to run the program for
        max_cycles: u64,
    },
    /// Execute an offchain job request (returns the signed job request and result)
    ExecuteOffchainJob {
        /// The guest binary path
        guest_binary_path: String,

        /// The hex encoded input to provide to the guest binary
        onchain_input: String,

        /// The hex encoded offchain input to provide to the guest binary
        offchain_input: String,

        /// The maximum number of cycles to run the program for
        max_cycles: u64,

        /// The address of the consumer contract
        consumer: String,

        /// The nonce of the offchain job request
        nonce: u64,

        /// The secret key to sign the job request
        secret: String,
    },
}

/// Run the CLI.
#[tokio::main]
pub async fn main() -> Result<()> {
    let secret = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let signer = create_signer(secret)?;

    match Command::parse() {
        Command::ExecuteOnchainJob { guest_binary_path, onchain_input, job_id, max_cycles } => {
            let job_id_decoded: [u8; 32] =
                hex::decode(job_id.strip_prefix("0x").unwrap_or(&job_id))?.try_into().unwrap();
            execute_onchain_job_ffi(
                guest_binary_path,
                hex::decode(onchain_input.strip_prefix("0x").unwrap_or(&onchain_input))?,
                job_id_decoded,
                max_cycles,
                &signer,
            )
            .await?
        }
        Command::ExecuteOffchainJob {
            guest_binary_path,
            onchain_input,
            offchain_input,
            max_cycles,
            consumer,
            nonce,
            secret,
        } => {
            execute_offchain_job_ffi(
                guest_binary_path,
                hex::decode(onchain_input.strip_prefix("0x").unwrap_or(&onchain_input))?,
                hex::decode(offchain_input.strip_prefix("0x").unwrap_or(&offchain_input))?,
                max_cycles,
                consumer,
                nonce,
                secret,
                &signer,
            )
            .await?
        }
    };

    Ok(())
}

/// Prints on stdio the Ethereum ABI and hex encoded result and signature.
async fn execute_onchain_job_ffi(
    elf_path: String,
    onchain_input: Vec<u8>,
    job_id: [u8; 32],
    max_cycles: u64,
    signer: &K256LocalSigner,
) -> Result<()> {
    let elf = std::fs::read(elf_path).unwrap();
    let program_id = compute_image_id(&elf)?;
    let program_id_bytes = program_id.as_bytes().try_into().expect("program id is 32 bytes");
    let journal = execute_onchain_job(&elf, &onchain_input, max_cycles)?;
    let result_with_metadata = abi_encode_result_with_metadata(
        job_id,
        onchain_input,
        max_cycles,
        program_id_bytes,
        journal,
    );

    let zkvm_operator_signature = sign_message(&result_with_metadata, signer).await?;

    let calldata =
        abi_encode_result_with_signature_calldata(result_with_metadata, zkvm_operator_signature);
    let output = hex::encode(calldata);

    // Forge test FFI calls expect hex encoded bytes sent to stdout
    print!("{output}");
    std::io::stdout().flush().context("failed to flush stdout buffer")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Prints on stdio the Ethereum ABI and hex encoded request and result
/// for an offchain job.
async fn execute_offchain_job_ffi(
    elf_path: String,
    onchain_input: Vec<u8>,
    offchain_input: Vec<u8>,
    max_cycles: u64,
    consumer: String,
    nonce: u64,
    secret: String,
    signer: &K256LocalSigner,
) -> Result<()> {
    let elf = std::fs::read(elf_path).unwrap();
    let program_id = compute_image_id(&elf)?;
    let program_id_bytes = program_id.as_bytes().try_into().expect("program id is 32 bytes");

    let onchain_input_hash = keccak256(onchain_input.clone());
    let offchain_input_hash = keccak256(offchain_input.clone());
    // Create a signed job request
    // This would normally be sent by the user/app signer, but we
    // construct it here to work with the foundry tests.
    let job_request = abi_encode_offchain_job_request(
        nonce,
        max_cycles,
        &consumer,
        program_id_bytes,
        onchain_input.clone(),
        offchain_input_hash.into(),
    );

    let offchain_signer = create_signer(&secret)?;
    let offchain_signer_signature = sign_message(&job_request, &offchain_signer).await?;

    let journal = execute_offchain_job(&elf, &onchain_input, &offchain_input, max_cycles)?;
    let job_id = get_job_id(nonce, Address::from_str(&consumer).unwrap());
    let offchain_result_with_metadata = abi_encode_offchain_result_with_metadata(
        job_id,
        onchain_input_hash.into(),
        offchain_input_hash.into(),
        max_cycles,
        program_id_bytes,
        journal,
    );
    let zkvm_operator_signature = sign_message(&offchain_result_with_metadata, signer).await?;

    let calldata = abi_encode_offchain_result_with_signature_calldata(
        offchain_result_with_metadata,
        zkvm_operator_signature,
        job_request,
        offchain_signer_signature,
    );
    let output = hex::encode(calldata);

    // Forge test FFI calls expect hex encoded bytes sent to stdout
    print!("{output}");
    std::io::stdout().flush().context("failed to flush stdout buffer")?;
    Ok(())
}

/// Generates journal for the given elf and input, for an onchain job.
fn execute_onchain_job(elf: &[u8], onchain_input: &[u8], max_cycles: u64) -> Result<Vec<u8>> {
    let onchain_input_len = onchain_input.len() as u32;

    let env = ExecutorEnv::builder()
        .session_limit(Some(max_cycles))
        .write(&onchain_input_len)?
        .write_slice(onchain_input)
        .build()?;

    let prover = LocalProver::new("locals only");
    let prove_info = prover.execute(env, elf)?;

    Ok(prove_info.journal.bytes)
}

/// Generates journal for the given elf and input, for an offchain job.
fn execute_offchain_job(
    elf: &[u8],
    onchain_input: &[u8],
    offchain_input: &[u8],
    max_cycles: u64,
) -> Result<Vec<u8>> {
    let onchain_input_len = onchain_input.len() as u32;
    let offchain_input_len = offchain_input.len() as u32;

    let env = ExecutorEnv::builder()
        .session_limit(Some(max_cycles))
        .write(&onchain_input_len)?
        .write_slice(onchain_input)
        .write(&offchain_input_len)?
        .write_slice(offchain_input)
        .build()?;

    let prover = LocalProver::new("locals only");
    let prove_info = prover.execute(env, elf)?;

    Ok(prove_info.journal.bytes)
}

/// The payload with result + signature that gets sent to the `JobManager` contract to decode.
///
/// tuple(ResultWithMetadata,Signature)
pub type ResultWithSignatureCalldata = sol! {
    tuple(bytes,bytes)
};

/// The payload with result + signature + job request + job request signature that gets sent
/// to the `JobManager` contract to decode.
///
/// tuple(ResultWithMetadata,Signature,OffchainJobRequest,OffchainJobRequestSignature)
pub type OffChainResultWithSignatureCalldata = sol! {
    tuple(bytes,bytes,bytes,bytes)
};

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed an onchain job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,OnchainInputHash,MaxCycles,VerifyingKey,RawOutput)
pub type ResultWithMetadata = sol! {
    tuple(bytes32,bytes32,uint64,bytes32,bytes)
};

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed an offchain job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,OnchainInputHash,OffchainInputHash,OffchainStateHash,MaxCycles,VerifyingKey,
/// `RawOutput`)
pub type OffChainResultWithMetadata = sol! {
    tuple(bytes32,bytes32,bytes32,uint64,bytes32,bytes)
};

/// The payload that gets signed by the entity sending an offchain job request.
/// This can be the user but the Consumer contract can decide who needs to
/// sign this request.
pub type OffchainJobRequest = sol! {
    tuple(uint64,uint64,address,bytes32,bytes,bytes32)
};

/// Returns ABI-encoded calldata with result and signature. This ABI-encoded response will be
/// sent to the `JobManager` contract.
pub fn abi_encode_result_with_signature_calldata(result: Vec<u8>, signature: Vec<u8>) -> Vec<u8> {
    ResultWithSignatureCalldata::abi_encode_params(&(result, signature))
}

/// Returns ABI-encoded calldata with result, signature, job request, and job request signature.
/// This ABI-encoded response will be sent to the `JobManager` contract.
pub fn abi_encode_offchain_result_with_signature_calldata(
    result: Vec<u8>,
    signature: Vec<u8>,
    job_request: Vec<u8>,
    job_request_signature: Vec<u8>,
) -> Vec<u8> {
    OffChainResultWithSignatureCalldata::abi_encode_params(&(
        result,
        signature,
        job_request,
        job_request_signature,
    ))
}

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the coprocessor operator.
pub fn abi_encode_result_with_metadata(
    job_id: [u8; 32],
    onchain_input: Vec<u8>,
    max_cycles: u64,
    program_verifying_key: &[u8; 32],
    raw_output: Vec<u8>,
) -> Vec<u8> {
    let onchain_input_hash = keccak256(onchain_input);
    ResultWithMetadata::abi_encode(&(
        job_id,
        onchain_input_hash,
        max_cycles,
        program_verifying_key,
        raw_output,
    ))
}

/// Returns an ABI-encoded offchain result with metadata. This ABI-encoded response will be
/// signed by the coprocessor operator.
pub fn abi_encode_offchain_result_with_metadata(
    job_id: [u8; 32],
    onchain_input_hash: [u8; 32],
    offchain_input_hash: [u8; 32],
    max_cycles: u64,
    program_verifying_key: &[u8; 32],
    raw_output: Vec<u8>,
) -> Vec<u8> {
    OffChainResultWithMetadata::abi_encode(&(
        job_id,
        onchain_input_hash,
        offchain_input_hash,
        max_cycles,
        program_verifying_key,
        raw_output,
    ))
}

/// Returns an ABI-encoded offchain job request. This ABI-encoded request can be
/// signed by the user sending the request, but the Consumer contract can
/// decide who this request should be signed by.
pub fn abi_encode_offchain_job_request(
    nonce: u64,
    max_cycles: u64,
    consumer: &str,
    program_verifying_key: &[u8; 32],
    onchain_input: Vec<u8>,
    offchain_input_hash: [u8; 32],
) -> Vec<u8> {
    OffchainJobRequest::abi_encode(&(
        nonce,
        max_cycles,
        Address::from_str(consumer).unwrap(),
        program_verifying_key,
        onchain_input,
        offchain_input_hash,
    ))
}

type NonceAndConsumer = sol! {
    tuple(uint64, address)
};

fn abi_encode_nonce_and_consumer(nonce: u64, consumer: Address) -> Vec<u8> {
    NonceAndConsumer::abi_encode_packed(&(nonce, consumer))
}

/// Returns the job ID hash for a given nonce and consumer address.
pub fn get_job_id(nonce: u64, consumer: Address) -> [u8; 32] {
    keccak256(abi_encode_nonce_and_consumer(nonce, consumer)).into()
}

fn create_signer(secret: &str) -> Result<LocalSigner<SigningKey>> {
    let hex = if let Some(stripped) = secret.strip_prefix("0x") { stripped } else { secret };
    let decoded = hex::decode(hex)?;
    let signer = K256LocalSigner::from_slice(&decoded)?;

    Ok(signer)
}

async fn sign_message(msg: &[u8], signer: &K256LocalSigner) -> Result<Vec<u8>> {
    let sig = signer.sign_message(msg).await?;

    Ok(sig.as_bytes().to_vec())
}
