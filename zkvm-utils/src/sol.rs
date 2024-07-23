use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{anyhow, bail, Context, Result};
use risc0_build::GuestListEntry;
use risc0_zkp::core::digest::Digest;

const IMAGE_ID_LIB_HEADER: &str = r#"pragma solidity ^0.8.13;

library ImageID {
"#;

/// Options for building and code generation.
#[derive(Debug, Clone, Default)]
#[non_exhaustive] // more options may be added in the future.
pub struct Options {
    /// Path the generated Solidity file with image ID information.
    pub image_id_sol_path: Option<PathBuf>,

    /// Path the generated Solidity file with deploy script for coprocessor contracts.
    pub deploy_script_path: Option<PathBuf>,
}

// Builder interface is provided to make it easy to add more intelligent default and additional
// options without breaking backwards compatibility in the future.
impl Options {
    /// Add a path to generate the Solidity file with image ID information.
    pub fn with_image_id_sol_path(mut self, path: impl AsRef<Path>) -> Self {
        self.image_id_sol_path = Some(path.as_ref().to_owned());
        self
    }

    /// Add a path to generate the Solidity file with deploy script for coprocessor contracts.
    pub fn with_deploy_script_path(mut self, path: impl AsRef<Path>) -> Self {
        self.deploy_script_path = Some(path.as_ref().to_owned());
        self
    }
}

/// Generate Solidity files for integrating a InfinityVM project.
pub fn generate_solidity_files(guests: &[GuestListEntry], opts: &Options) -> Result<()> {
    // Skip Solidity source files generation if RISC0_SKIP_BUILD is enabled.
    if env::var("RISC0_SKIP_BUILD").is_ok() {
        return Ok(());
    }

    let image_id_file_path = opts
        .image_id_sol_path
        .as_ref()
        .ok_or(anyhow!("path for image ID Solidity file must be provided"))?;
    fs::write(image_id_file_path, generate_image_id_sol(guests)?)
        .with_context(|| format!("failed to save changes to {}", image_id_file_path.display()))?;

    let deploy_script_path = opts
        .deploy_script_path
        .as_ref()
        .ok_or(anyhow!("path for deploy script Solidity file must be provided"))?;
    fs::write(deploy_script_path, generate_deploy_script(guests)?)
        .with_context(|| format!("failed to save changes to {}", deploy_script_path.display()))?;

    Ok(())
}

/// Generate source code for a Solidity library containing image IDs for the given guest programs.
pub fn generate_image_id_sol(guests: &[GuestListEntry]) -> Result<Vec<u8>> {
    // Assemble a list of image IDs.
    let image_ids: Vec<_> = guests
        .iter()
        .map(|guest| {
            let name = guest.name.to_uppercase().replace('-', "_");
            let image_id = hex::encode(Digest::from(guest.image_id));
            format!("bytes32 public constant {name}_ID = bytes32(0x{image_id});")
        })
        .collect();

    let image_id_lines = image_ids.join("\n");

    // Building the final image_ID file content.
    let file_content = format!("{IMAGE_ID_LIB_HEADER}\n{image_id_lines}\n}}");
    forge_fmt(file_content.as_bytes()).context("failed to format image ID file")
}

/// Generate source code for Solidity deploy script for coprocessor contracts
pub fn generate_deploy_script(guests: &[GuestListEntry]) -> Result<Vec<u8>> {
    // Generate the code to set ELF paths
    let elf_entries: Vec<_> = guests
        .iter()
        .map(|guest| {
            let image_id = hex::encode(Digest::from(guest.image_id));
            let elf_path = guest.path.to_string();
            format!("jobManager.setElfPath(bytes32(0x{}), \"{}\");", image_id, elf_path)
        })
        .collect();

    let set_elf_paths_code = elf_entries.join("\n");

    let file_content = format!(
        r#"
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {{Script, console}} from "forge-std/Script.sol";
import {{JobManager}} from "../src/JobManager.sol";
import {{IJobManager}} from "../src/IJobManager.sol";
import {{Consumer}} from "../src/Consumer.sol";
import {{ExampleConsumer}} from "../src/ExampleConsumer.sol";
import {{Utils}} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";

// To deploy and verify:
// forge script CoprocessorDeployer.s.sol:CoprocessorDeployer --sig "deployCoprocessorContracts(address relayer, address coprocessorOperator)" $RELAYER $COPROCESSOR_OPERATOR --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract CoprocessorDeployer is Script, Utils {{

    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    ExampleConsumer public consumer;

    function deployCoprocessorContracts(address relayer, address coprocessorOperator) public {{
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

        consumer = new ExampleConsumer(address(jobManager));

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
        bail!("forge fmt on image ID file content exited with status {}", fmt_out.status,);
    }

    Ok(fmt_out.stdout)
}
