//! CLOB execution engine.

use crate::db::{
    tables::{ClobStateTable, GlobalIndexTable, RequestTable, ResponseTable},
    PROCESSED_GLOBAL_INDEX_KEY, SEEN_GLOBAL_INDEX_KEY,
};
use clob_core::{
    api::{ApiResponse, Request},
    tick, ClobState,
};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
use std::sync::Arc;
use tokio::{
    sync::{mpsc::Receiver, oneshot},
    task::JoinSet,
};

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
pub async fn run_engine<D>(
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    db: Arc<D>,
) where
    D: Database + 'static,
{
    let (mut global_index, mut state) = read_start_up_values(Arc::clone(&db));

    // TODO add logic to clear the joinset
    let mut handles = JoinSet::new();

    loop {
        global_index += 1;

        // TODO: refactor so this recieves a nonce and then uses that nonce to read from DB
        // This should help ensure ordering
        let (request, response_sender) = receiver.recv().await.expect("todo");
        println!("engine: {:?}, response_sender: {:?}", request, response_sender);

        // In background thread persist the index and request
        let request2 = request.clone();
        let db2 = Arc::clone(&db);
        handles.spawn(async move {
            let tx = db2.tx_mut().expect("todo");
            tx.put::<GlobalIndexTable>(SEEN_GLOBAL_INDEX_KEY, global_index).expect("todo");
            tx.put::<RequestTable>(global_index, request2).expect("todo");
            tx.commit().expect("todo");
        });

        // TODO: logic to switch between zkvm, vs plain code
        let (response, post_state) = tick(request, state).expect("TODO");

        // In a background task persist: processed index, response, and new state.
        // TODO: cloning entire state is not ideal, would be better to somehow just apply state
        // diffs.
        let post_state2 = post_state.clone();
        let response2 = response.clone();
        let db2 = Arc::clone(&db);
        handles.spawn(async move {
            let tx = db2.tx_mut().expect("todo");
            tx.put::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY, global_index).expect("todo");
            tx.put::<ResponseTable>(global_index, response2).expect("todo");
            tx.put::<ClobStateTable>(global_index, post_state2).expect("todo");
            tx.commit().expect("todo");
        });

        let api_response = ApiResponse { response, global_index };
        println!("engine: api_response={:?}", api_response);

        response_sender.send(api_response).expect("todo");

        state = post_state;
    }
}
