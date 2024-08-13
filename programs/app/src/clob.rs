use clob_core::{api::Request, tick, State};
use risc0_zkvm::guest::env::{self, Read};

fn main() {
    // Read the input data for this application.
    let mut input_bytes = Vec::<u8>::new();
    env::stdin().read_slice(&mut input_bytes);

    let (request, state): (Request, State) = borsh::from_slice(&input_bytes).expect("todo");

    let response = {
        let response = tick(request, state).expect("todo");
        borsh::to_vec(&response).expect("todo")
    };

    env::commit_slice(&response);
}
