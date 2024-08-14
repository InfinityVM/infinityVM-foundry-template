//! The CLOB node.

use axum::{extract::State as ExtractState, Json};
use clob_core::api::{
    AddOrderRequest, ApiResponse, CancelOrderRequest, DepositRequest, Request, WithdrawRequest,
};
use tokio::sync::{mpsc::Sender, oneshot};

pub mod db;
pub mod engine;

/// Stateful parts of rest server
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Engine send channel handle.
    pub engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    // TODO: read only db handle so we can read back order status and return order book view
}

/// Run the HTTP server.
pub async fn http_listen(state: ServerState, listen_address: &str) {
    let app = axum::Router::new()
        .route("/deposit", axum::routing::post(deposit))
        .route("/withdraw", axum::routing::post(withdraw))
        .route("/orders", axum::routing::post(add_order))
        .route("/cancel", axum::routing::post(cancel))
        // TODO: we
        // .route("/status", axum::routing::post(order_status))
        .with_state(state);

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
