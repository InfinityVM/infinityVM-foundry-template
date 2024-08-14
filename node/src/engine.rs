use clob_core::{api::Request, tick, State as ClobState};
use tokio::sync::{mpsc::Receiver, oneshot};

pub async fn run_engine(
    mut state: ClobState,
    mut receiver: Receiver<(Request, oneshot::Sender<u64>)>,
    mut global_idx: u64,
) {
    loop {
        // TODO: refactor so this recieves a nonce and then uses that nonce to read from DB
        // This should help ensure ordering
        let (request, sender) = receiver.recv().await.expect("todo");
        let cur_idx = global_idx;
        global_idx += 1;

        // In new background thread:
        // - relay back the index of the transaction so we can return it as a preconf
        // - write highest seen gidx to db
        // - write request to db, keyed by gidx
        sender.send(global_idx).expect("todo");

        // TODO: logic to switch between zkvm, vs plain code
        let (result, post_state) = tick(request, state).expect("TODO");

        println!("cur_idx={:?}, result={:?}", cur_idx, result);
        // In new background thread
        // Record stuff in DB
        // - the request, result and global nonce
        // - the hash of the state after the transition
        // - maybe the whole state?

        state = post_state
    }
}
