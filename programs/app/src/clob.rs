//! ZKVM program for running the tick

use clob_core::{api::Request, tick, ClobState};
use risc0_zkvm::guest::env;

fn main() {
    // Read the input data for this application.
    let len: u32 = env::read();
    let mut buf = vec![0; len as usize];
    env::read_slice(&mut buf);

    let (request, state): (Request, ClobState) = borsh::from_slice(&buf).expect("todo");

    let response = {
        let response = tick(request, state).expect("todo");
        borsh::to_vec(&response).expect("todo")
    };

    env::commit_slice(&response);
}
