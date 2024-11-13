#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::{
    primitives::{U256},
};
use alloy_sol_types::{sol, SolType, SolValue};

type NumberWithSquareRoot = sol! {
    tuple(uint256,uint256)
};

fn main() {
    // This application only uses onchain input. We read the onchain input here.
    let onchain_input = sp1_zkvm::io::read_vec();

    // Decode and parse the input
    let number = <U256>::abi_decode(&onchain_input, true).unwrap();

    // Calculate square root
    let square_root = number.root(2);
    
    // Commit the journal that will be received by the application contract.
    // Journal is encoded using Solidity ABI for easy decoding in the app contract.
    sp1_zkvm::io::commit_slice(NumberWithSquareRoot::abi_encode(&(number, square_root)).as_slice());
}
