use clob_core::{State, api::Request, tick};
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver

async fn run_engine(mut state: State, receiver: Receiver<Request>, mut global_idx: u64) {
  loop {
    let request = self.receiver.recv().await.expect("todo");
    let cur_idx = global_idx;
    global_idx += 1;
    (result, post_state) = tick(request, state);

    println!("cur_idx={}, result={:?}", cur_idx, result);
    // Record stuff in DB
    // - the request, result and global nonce
    // - the hash of the state after the transition

    state = post_state
  }
}
