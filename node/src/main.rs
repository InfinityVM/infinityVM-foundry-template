use clob_core::State as ClobState;
use node::{engine, http_listen, ServerState};
use tokio::join;

// Small for now to get to failure cases quicker
const CHANEL_SIZE: usize = 32;

#[tokio::main]
async fn main() {
    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(CHANEL_SIZE);

    let server_state = ServerState { engine_sender };
    let http_listen_address = "127.0.0.1:3001";

    let server_handle = http_listen(server_state, http_listen_address);

    // TODO read in from DB
    let clob_state = ClobState::default();
    let global_idx = 0;

    let engine_handle = engine::run_engine(clob_state, engine_receiver, global_idx);

    let (_server_result, _engine_result) = join!(server_handle, engine_handle);
}
