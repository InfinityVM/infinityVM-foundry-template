use sp1_build::{build_program_with_args, BuildArgs};
use zkvm_utils::sol::{generate_solidity_files, Options};

// Paths where the generated Solidity files will be written.
const SOLIDITY_PROGRAM_ID_PATH: &str = "../contracts/src/ProgramID.sol";
const SOLIDITY_DEPLOY_SCRIPT_PATH: &str = "../contracts/script/Deployer.s.sol";

// Add your zkVM programs here.
const PROGRAM_NAMES: &[&str] = &["square-root"];

fn main() {
    let programs: Vec<String> = PROGRAM_NAMES.to_vec().iter().map(|s| s.to_string()).collect();

    // For each program, build the ELF.
    for program in programs.clone() {
        let args = BuildArgs {
            elf_name: program.clone(),
            output_directory: format!("programs/{}/elf", program),
            ..Default::default()
        };
        build_program_with_args(&program, args);
    }

    // Generate Solidity source files for use with Forge.
    let solidity_opts = Options::default()
        .with_program_id_sol_path(SOLIDITY_PROGRAM_ID_PATH)
        .with_deploy_script_path(SOLIDITY_DEPLOY_SCRIPT_PATH);

    generate_solidity_files(programs, &solidity_opts).unwrap();
}
