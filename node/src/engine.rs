//! CLOB execution engine.

use clob_core::{
    api::{ApiResponse, Request},
    tick, ClobState,
};
use tokio::sync::{mpsc::Receiver, oneshot};

/// Run the CLOB execution engine
pub async fn run_engine(
    mut state: ClobState,
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    mut global_index: u64,
) {
    loop {
        // TODO: refactor so this recieves a nonce and then uses that nonce to read from DB
        // This should help ensure ordering
        let (request, response_sender) = receiver.recv().await.expect("todo");
        println!("engine: {:?}, response_sender: {:?}", request, response_sender);

        // In new background thread:
        //
        // - write request to db, keyed by gidx

        // TODO: logic to switch between zkvm, vs plain code
        let (response, post_state) = tick(request, state).expect("TODO");

        let api_response = ApiResponse { response, global_index };
        println!("engine: api_response={:?}", api_response);

        response_sender.send(api_response).expect("todo");

        // In new background thread
        // Record stuff in DB
        // - the request, result and global nonce
        // - the hash of the state after the transition
        // - maybe the whole state?

        state = post_state;
        global_index += 1;
    }
}
