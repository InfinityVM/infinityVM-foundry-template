use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use alloy::primitives::hex;
use anyhow::{anyhow, bail, Context, Result};
use ivm_zkvm::Zkvm;

const PROGRAM_ID_LIB_HEADER: &str = r#"pragma solidity ^0.8.13;

library ProgramID {
"#;

/// Metadata for a program.
#[derive(Debug, Clone)]
pub struct ProgramMetadata {
    /// Name of the program.
    pub name: String,
    /// Hex-encoded program ID.
    pub program_id_hex: String,
    /// Path to the ELF file.
    pub elf_path: String,
}

/// Options for building and code generation.
#[derive(Debug, Clone, Default)]
#[non_exhaustive] // more options may be added in the future.
pub struct Options {
    /// Path the generated Solidity file with program ID information.
    pub program_id_sol_path: Option<PathBuf>,

    /// Path the generated Solidity file with deploy script for coprocessor contracts.
    pub deploy_script_path: Option<PathBuf>,
}

// Builder interface is provided to make it easy to add more intelligent default and additional
// options without breaking backwards compatibility in the future.
impl Options {
    /// Add a path to generate the Solidity file with program ID information.
    pub fn with_program_id_sol_path(mut self, path: impl AsRef<Path>) -> Self {
        self.program_id_sol_path = Some(path.as_ref().to_owned());
        self
    }

    /// Add a path to generate the Solidity file with deploy script for coprocessor contracts.
    pub fn with_deploy_script_path(mut self, path: impl AsRef<Path>) -> Self {
        self.deploy_script_path = Some(path.as_ref().to_owned());
        self
    }
}

/// Generate Solidity files for testing a consumer app with `InfinityVM`.
pub fn generate_solidity_files(program_names: Vec<String>, opts: &Options) -> Result<()> {
    // Skip Solidity source files generation if INFINITY_SKIP_BUILD is enabled.
    if env::var("INFINITY_SKIP_BUILD").is_ok() {
        return Ok(());
    }

    // Construct program metadata.
    let programs: Vec<ProgramMetadata> = program_names
        .iter()
        .map(|name| {
            let elf_path = format!("elf/{name}");
            let elf = std::fs::read(elf_path).unwrap();
            let program_id = ivm_zkvm::Sp1.derive_verifying_key(&elf).unwrap();
            let elf_path_sol = format!("programs/elf/{name}");
            ProgramMetadata {
                name: name.clone(),
                program_id_hex: hex::encode(program_id),
                elf_path: elf_path_sol,
            }
        })
        .collect();

    let program_id_file_path = opts
        .program_id_sol_path
        .as_ref()
        .ok_or_else(|| anyhow!("path for program ID Solidity file must be provided"))?;
    fs::write(program_id_file_path, generate_program_id_sol(&programs)?)
        .with_context(|| format!("failed to save changes to {}", program_id_file_path.display()))?;

    let deploy_script_path = opts
        .deploy_script_path
        .as_ref()
        .ok_or_else(|| anyhow!("path for deploy script Solidity file must be provided"))?;
    fs::write(deploy_script_path, generate_deploy_script(&programs)?)
        .with_context(|| format!("failed to save changes to {}", deploy_script_path.display()))?;

    Ok(())
}

/// Generate source code for a Solidity library containing program IDs for the given programs.
pub fn generate_program_id_sol(programs: &[ProgramMetadata]) -> Result<Vec<u8>> {
    // Assemble a list of program IDs.
    let program_ids: Vec<_> = programs
        .iter()
        .map(|program| {
            let name = program.name.to_uppercase().replace('-', "_");
            let program_id = program.program_id_hex.clone();
            format!("bytes32 public constant {name}_ID = bytes32(0x{program_id});")
        })
        .collect();

    let program_id_lines = program_ids.join("\n");

    // Building the final program_ID file content.
    let file_content = format!("{PROGRAM_ID_LIB_HEADER}\n{program_id_lines}\n}}");
    forge_fmt(file_content.as_bytes()).context("failed to format program ID file")
}

/// Generate source code for Solidity deploy script for coprocessor contracts
pub fn generate_deploy_script(programs: &[ProgramMetadata]) -> Result<Vec<u8>> {
    // Generate the code to set ELF paths
    let relative_elf_path_prefix = "programs/elf/";
    let elf_entries: Vec<_> = programs
        .iter()
        .map(|program| {
            let program_id = program.program_id_hex.clone();
            let absolute_elf_path = program.elf_path.to_string();
            let relative_elf_path =
                if let Some(pos) = absolute_elf_path.find(relative_elf_path_prefix) {
                    &absolute_elf_path[pos..]
                } else {
                    absolute_elf_path.as_str()
                };

            format!("jobManager.setElfPath(bytes32(0x{}), \"{}\");", program_id, relative_elf_path)
        })
        .collect();

    let set_elf_paths_code = elf_entries.join("\n");

    let file_content = format!(
        r#"
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {{Script, console}} from "forge-std/Script.sol";
import {{JobManager}} from "../src/coprocessor/JobManager.sol";
import {{IJobManager}} from "../src/coprocessor/IJobManager.sol";
import {{Consumer}} from "../src/coprocessor/Consumer.sol";
import {{SquareRootConsumer}} from "../src/SquareRootConsumer.sol";
import {{Utils}} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";

// To deploy and verify:
// forge script Deployer.s.sol:Deployer --sig "deployContracts(address relayer, address coprocessorOperator, address offchainRequestSigner, uint64 initialMaxNonce)" $RELAYER $COPROCESSOR_OPERATOR $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract Deployer is Script, Utils {{

    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    SquareRootConsumer public consumer;

    function deployContracts(address relayer, address coprocessorOperator, address offchainRequestSigner, uint64 initialMaxNonce) public {{
        vm.startBroadcast();
        // deploy proxy admin for ability to upgrade proxy contracts
        coprocessorProxyAdmin = new ProxyAdmin();

        jobManagerImplementation = new JobManager();
        jobManager = JobManager(
            address(
                new TransparentUpgradeableProxy(
                    address(jobManagerImplementation),
                    address(coprocessorProxyAdmin),
                    abi.encodeWithSelector(
                        jobManager.initializeJobManager.selector,
                        msg.sender,
                        relayer,
                        coprocessorOperator
                    )
                )
            )
        );

        consumer = new SquareRootConsumer(address(jobManager), offchainRequestSigner, initialMaxNonce);

        // Set ELF paths
        {set_elf_paths_code}

        vm.stopBroadcast();
    }}
}}
"#,
        set_elf_paths_code = set_elf_paths_code
    );

    forge_fmt(file_content.as_bytes()).context("failed to format deploy script file")
}

/// Uses forge fmt as a subprocess to format the given Solidity source.
fn forge_fmt(src: &[u8]) -> Result<Vec<u8>> {
    // Spawn `forge fmt`
    let mut fmt_proc = Command::new("forge")
        .args(["fmt", "-", "--raw"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to spawn forge fmt")?;

    // Write the source code as bytes to stdin.
    fmt_proc
        .stdin
        .take()
        .context("failed to take forge fmt stdin handle")?
        .write_all(src)
        .context("failed to write to forge fmt stdin")?;

    let fmt_out = fmt_proc.wait_with_output().context("failed to run forge fmt")?;

    if !fmt_out.status.success() {
        bail!("forge fmt on program ID file content exited with status {}", fmt_out.status,);
    }

    Ok(fmt_out.stdout)
}
