use std::io::Read;

use alloy_primitives::U256;
use alloy_sol_types::{sol, SolType, SolValue};
use risc0_zkvm::guest::env;

type NumberWithSquareRoot = sol! {
    tuple(uint256,uint256)
};

fn main() {
    // Read the input data for this application.
    let mut input_bytes = Vec::<u8>::new();
    env::stdin().read_to_end(&mut input_bytes).unwrap();
    // Decode and parse the input
    let number = <U256>::abi_decode(&input_bytes, true).unwrap();

    // Calculate square root
    let square_root = number.root(2);

    // Commit the journal that will be received by the application contract.
    // Journal is encoded using Solidity ABI for easy decoding in the app contract.
    env::commit_slice(NumberWithSquareRoot::abi_encode(&(number, square_root)).as_slice());
}
