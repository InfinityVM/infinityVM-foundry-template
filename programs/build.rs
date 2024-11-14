use std::fs;

use sp1_build::{build_program_with_args, BuildArgs};
use zkvm_utils::sol::{generate_solidity_files, Options};

// Paths where the generated Solidity files will be written.
const SOLIDITY_PROGRAM_ID_PATH: &str = "../contracts/src/ProgramID.sol";
const SOLIDITY_DEPLOY_SCRIPT_PATH: &str = "../contracts/script/Deployer.s.sol";

fn main() {
    // Get a list of all programs.
    let entries = fs::read_dir(".").unwrap();
    let mut programs = Vec::new();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name() {
                let dir_name = dir_name.to_string_lossy();
                if dir_name != "src" && dir_name != "elf" {
                    programs.push(dir_name.to_string());
                }
            }
        }
    }

    if programs.is_empty() {
        panic!("No programs found in the current directory");
    }

    // For each program, build the ELF.
    for program in programs {
        let args = BuildArgs {
            elf_name: program.clone(),
            output_directory: "programs/elf".to_string(),
            ..Default::default()
        };
        build_program_with_args(&program, args);
    }

    // Generate Solidity source files for use with Forge.
    let solidity_opts = Options::default()
        .with_program_id_sol_path(SOLIDITY_PROGRAM_ID_PATH)
        .with_deploy_script_path(SOLIDITY_DEPLOY_SCRIPT_PATH);

    // generate_solidity_files(guests.as_slice(), &solidity_opts).unwrap();
}
