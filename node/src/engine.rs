//! CLOB execution engine.

use crate::db::{
    tables::{ClobStateTable, GlobalIndexTable},
    PROCESSED_GLOBAL_INDEX_KEY,
};
use clob_core::{
    api::{ApiResponse, Request},
    tick, ClobState,
};
use reth_db::{transaction::DbTx, Database};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};
const START_GLOBAL_INDEX: u64 = 0;

fn read_start_up_values<D: Database + 'static>(db: Arc<D>) -> (u64, ClobState) {
    let tx = db.tx().expect("todo");

    let global_index = tx
        .get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY)
        .expect("todo: db errors")
        .unwrap_or(START_GLOBAL_INDEX);

    let clob_state = if global_index == START_GLOBAL_INDEX {
        // Initialize clob state if we haven't processed anything
        ClobState::default()
    } else {
        tx.get::<ClobStateTable>(global_index)
            .expect("todo: db errors")
            .expect("todo: could not find state when some was expected")
    };

    tx.commit().expect("todo");

    (global_index, clob_state)
}

/// Run the CLOB execution engine
/// 
pub async fn run_engine<D>(
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    db: Arc<D>,
) where
    D: Database + 'static,
{
    let (mut global_index, mut state) = read_start_up_values(Arc::clone(&db));

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
