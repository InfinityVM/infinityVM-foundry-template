// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::Read;

use alloy_primitives::{U256, Address};
use alloy_sol_types::{sol, SolType};
use risc0_zkvm::guest::env;

type AddressEncodeable = sol! {
    address
};

type AddressWithBalance = sol! {
    tuple(address,uint256)
};

fn abi_encode_address_with_balance(address: Address, balance: U256) -> Vec<u8> {
    AddressWithBalance::abi_encode(&(address, balance))
}

fn main() {
    // Read the input data for this application.
    let mut input_bytes = Vec::<u8>::new();
    env::stdin().read_to_end(&mut input_bytes).unwrap();
    // Decode and parse the input
    let decoded_address: alloy_sol_types::private::Address = AddressEncodeable::abi_decode(&input_bytes, true).unwrap();

    let address_bytes: [u8; 20] = decoded_address.into();
    let address: Address = Address::from(address_bytes);

    // set balance
    let balance = U256::from(10000000);

    // Commit the journal that will be received by the application contract.
    // Journal is encoded using Solidity ABI for easy decoding in the app contract.
    env::commit_slice(abi_encode_address_with_balance(address, balance).as_slice());
}
