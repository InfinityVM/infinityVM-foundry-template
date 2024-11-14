use zkvm_utils::sol::{generate_solidity_files, Options};
use sp1_build::{build_program_with_args, BuildArgs};

// Paths where the generated Solidity files will be written.
const SOLIDITY_PROGRAM_ID_PATH: &str = "../contracts/src/ProgramID.sol";
const SOLIDITY_DEPLOY_SCRIPT_PATH: &str = "../contracts/script/Deployer.s.sol";

fn main() {
    // let guests = risc0_build::embed_methods();

    // Generate Solidity source files for use with Forge.
    let solidity_opts = Options::default()
        .with_program_id_sol_path(SOLIDITY_PROGRAM_ID_PATH)
        .with_deploy_script_path(SOLIDITY_DEPLOY_SCRIPT_PATH);

    let args = BuildArgs {
        elf_name: "square-root".to_string(),
        output_directory: "programs/elf".to_string(),
        ..Default::default()
    };
    build_program_with_args("./square-root", args);

    // generate_solidity_files(guests.as_slice(), &solidity_opts).unwrap();
}
