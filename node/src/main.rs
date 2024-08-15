//! The Infinity CLOB node binary.

use clob_core::ClobState;
use node::{engine, http_listen, ServerState};
use std::sync::Arc;
use tokio::join;

// Small for now to get to failure cases quicker
const CHANEL_SIZE: usize = 32;
const DB_DIR: &str = "./tmp-data-dir/dev/db";

#[tokio::main]
async fn main() {
    let db = node::db::init_db(DB_DIR).expect("todo");
    let db = Arc::new(db);

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(CHANEL_SIZE);

    let server_state = ServerState { engine_sender };
    let http_listen_address = "127.0.0.1:3001";

    let server_handle = http_listen(server_state, http_listen_address);

    let engine_handle = engine::run_engine(engine_receiver, db);

    let (_server_result, _engine_result) = join!(server_handle, engine_handle);
}
