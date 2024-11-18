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
use ivm_abi::{abi_encode_offchain_job_request, get_job_id, JobParams};
use ivm_proto::VmType;
use ivm_zkvm::Zkvm;
use k256::ecdsa::SigningKey;
use ivm_zkvm_executor::service::ZkvmExecutorService;

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
    let zkvm_executor = ZkvmExecutorService::new(signer);

    match Command::parse() {
        Command::ExecuteOnchainJob { guest_binary_path, onchain_input, job_id, max_cycles } => {
            let job_id_decoded: [u8; 32] =
                hex::decode(job_id.strip_prefix("0x").unwrap_or(&job_id))?.try_into().unwrap();
            execute_onchain_job_ffi(
                guest_binary_path,
                hex::decode(onchain_input.strip_prefix("0x").unwrap_or(&onchain_input))?,
                job_id_decoded,
                max_cycles,
                &zkvm_executor,
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
                &zkvm_executor,
            )
            .await?
        }
    };

    Ok(())
}

/// Prints on stdio the Ethereum ABI and hex encoded result and signature
/// for an onchain job.
async fn execute_onchain_job_ffi(
    elf_path: String,
    onchain_input: Vec<u8>,
    job_id: [u8; 32],
    max_cycles: u64,
    zkvm_executor: &ZkvmExecutorService<LocalSigner<SigningKey>>,
) -> Result<()> {
    let elf = std::fs::read(elf_path).unwrap();
    // TODO: pass this in instead of re-deriving it?
    let program_id = ivm_zkvm::Sp1.derive_verifying_key(&elf)?;

    let (result_with_metadata, zkvm_operator_signature) = zkvm_executor
        .execute_onchain_job(job_id, max_cycles, program_id, onchain_input, elf, VmType::Sp1)
        .await
        .unwrap();

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
    zkvm_executor: &ZkvmExecutorService<LocalSigner<SigningKey>>,
) -> Result<()> {
    // print current directory
    let elf = std::fs::read(elf_path).unwrap();
    let program_id = ivm_zkvm::Sp1.derive_verifying_key(&elf)?;
    let offchain_input_hash = keccak256(offchain_input.clone());

    // Create a signed job request
    // This would normally be sent by the user/app signer, but we
    // construct it here to work with the foundry tests.
    let job_params = JobParams {
        nonce,
        max_cycles,
        consumer_address: **Address::from_str(&consumer).unwrap(),
        program_id: &program_id,
        onchain_input: &onchain_input,
        offchain_input_hash: offchain_input_hash.into(),
    };
    let job_request = abi_encode_offchain_job_request(job_params);
    let offchain_signer = create_signer(&secret)?;
    let offchain_signer_signature = sign_message(&job_request, &offchain_signer).await?;

    let job_id = get_job_id(nonce, Address::from_str(&consumer).unwrap());
    let (offchain_result_with_metadata, zkvm_operator_signature, _) = zkvm_executor
        .execute_offchain_job(
            job_id,
            max_cycles,
            program_id,
            onchain_input,
            offchain_input,
            elf,
            VmType::Sp1,
        )
        .await
        .unwrap();

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
