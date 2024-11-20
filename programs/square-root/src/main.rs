#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::{
    primitives::U256,
    sol,
    sol_types::{SolType, SolValue},
};

sol! {
    struct NumberWithSquareRoot {
        uint256 number;
        uint256 square_root;
    }
}

fn main() {
    // This application only uses onchain input. We read the onchain input here.
    let onchain_input = sp1_zkvm::io::read_vec();
    // Decode and parse the input
    let number = <U256>::abi_decode(&onchain_input, true).unwrap();

    // Calculate square root
    let square_root = number.root(2);

    // Commit the output that will be received by the application contract.
    // Output is encoded using Solidity ABI for easy decoding in the app contract.
    let number_with_square_root = NumberWithSquareRoot { number, square_root };
    sp1_zkvm::io::commit_slice(
        <NumberWithSquareRoot as SolType>::abi_encode(&number_with_square_root).as_slice(),
    );
}
