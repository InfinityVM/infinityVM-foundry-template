//! The CLOB node.

use crate::{
    db::{
        tables::{ClobStateTable, GlobalIndexTable},
        PROCESSED_GLOBAL_INDEX_KEY,
    },
    engine::START_GLOBAL_INDEX,
};
use axum::{extract::State as ExtractState, Json, Router};
use clob_core::{
    api::{
        AddOrderRequest, ApiResponse, CancelOrderRequest, DepositRequest, Request, WithdrawRequest,
    },
    ClobState,
};
use reth_db::{transaction::DbTx, Database, DatabaseEnv};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};

pub mod db;
pub mod engine;

/// Stateful parts of rest server
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Engine send channel handle.
    pub engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    /// The database
    pub db: Arc<DatabaseEnv>,
}

fn app(state: ServerState) -> Router {
    axum::Router::new()
        .route("/deposit", axum::routing::post(deposit))
        .route("/withdraw", axum::routing::post(withdraw))
        .route("/orders", axum::routing::post(add_order))
        .route("/cancel", axum::routing::post(cancel))
        .route("/clob-state", axum::routing::get(clob_state))
        .with_state(state)
}

/// Run the HTTP server.
pub async fn http_listen(state: ServerState, listen_address: &str) {
    let app = app(state);

    let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn deposit(
    ExtractState(state): ExtractState<ServerState>,
    Json(req): Json<DepositRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::Deposit(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    println!("deposit: response: {:?}", resp);

    Json(resp)
}

async fn withdraw(
    ExtractState(state): ExtractState<ServerState>,
    Json(req): Json<WithdrawRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::Withdraw(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    println!("withdraw: response: {:?}", resp);

    Json(resp)
}

async fn add_order(
    ExtractState(state): ExtractState<ServerState>,
    Json(req): Json<AddOrderRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::AddOrder(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    println!("add_order: response: {:?}", resp);

    Json(resp)
}

async fn cancel(
    ExtractState(state): ExtractState<ServerState>,
    Json(req): Json<CancelOrderRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::CancelOrder(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    println!("cancel: response: {:?}", resp);

    Json(resp)
}

async fn clob_state(ExtractState(state): ExtractState<ServerState>) -> Json<ClobState> {
    let tx = state.db.tx().expect("todo");

    let global_index = tx
        .get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY)
        .expect("todo: db errors")
        .unwrap_or(START_GLOBAL_INDEX);

    let clob_state = if global_index == START_GLOBAL_INDEX {
        ClobState::default()
    } else {
        tx.get::<ClobStateTable>(global_index)
            .expect("todo: db errors")
            .expect("todo: could not find state when some was expected")
            .0
    };
    tx.commit().expect("todo");

    Json(clob_state)
}

// ref for testing: https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs
#[cfg(test)]
mod tests {}
