//! CLOB execution engine.

use crate::db::{
    models::{ClobStateModel, RequestModel, ResponseModel},
    tables::{ClobStateTable, GlobalIndexTable, RequestTable, ResponseTable},
    PROCESSED_GLOBAL_INDEX_KEY, SEEN_GLOBAL_INDEX_KEY,
};
use clob_core::{
    api::{ApiResponse, Request, Response},
    tick, ClobState,
};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
#[cfg(feature = "zkvm-execute")]
use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};
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
            .0
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

    #[cfg(feature = "zkvm-execute")]
    let zkvm_executor = LocalProver::new("locals only");

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
            tx.put::<RequestTable>(global_index, RequestModel(request2)).expect("todo");
            tx.commit().expect("todo");
        });

        // TODO: logic to switch between zkvm, vs plain code
        #[cfg(feature = "zkvm-execute")]
        let (response, post_state) = {
            let zkvm_input =
                borsh::to_vec(&(&request, &state)).expect("borsh serialize works. qed.");
            let env = ExecutorEnv::builder().write_slice(&zkvm_input).build().unwrap();
            let execute_info = zkvm_executor.execute(env, programs::CLOB_ELF).unwrap();

            let (z_response, z_post_state): (Response, ClobState) =
                borsh::from_slice(&execute_info.journal.bytes).expect("todo");

            let (n_response, n_post_state) = tick(request, state).expect("TODO");

            assert_eq!(z_response, n_response);
            assert_eq!(z_post_state, n_post_state);

            (n_response, n_post_state)
        };
        #[cfg(not(feature = "zkvm-execute"))]
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
            tx.put::<ResponseTable>(global_index, ResponseModel(response2)).expect("todo");
            tx.put::<ClobStateTable>(global_index, ClobStateModel(post_state2)).expect("todo");
            tx.commit().expect("todo");
        });

        let api_response = ApiResponse { response, global_index };
        println!("engine: api_response={:?}", api_response);

        response_sender.send(api_response).expect("todo");

        state = post_state;
    }
}
