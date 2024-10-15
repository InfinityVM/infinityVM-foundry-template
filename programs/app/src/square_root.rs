use alloy_primitives::U256;
use alloy_sol_types::{sol, SolType, SolValue};
use risc0_zkvm::guest::env;

type NumberWithSquareRoot = sol! {
    tuple(uint256,uint256)
};

fn main() {
    // This application only uses onchain input. We read the onchain input here.
    let onchain_input_len: u32 = env::read();
    let mut input_bytes = vec![0; onchain_input_len as usize];
    env::read_slice(&mut input_bytes);

    // Decode and parse the input
    let number = <U256>::abi_decode(&input_bytes, true).unwrap();

    // Calculate square root
    let square_root = number.root(2);

    // Commit the journal that will be received by the application contract.
    // Journal is encoded using Solidity ABI for easy decoding in the app contract.
    env::commit_slice(NumberWithSquareRoot::abi_encode(&(number, square_root)).as_slice());
}
