use axum::{extract::State as ExtractState, Json};
use clob_core::api::{DepositRequest, DepositResponse, Request, WithdrawRequest, WithdrawResponse};
use tokio::sync::{mpsc::Sender, oneshot};

pub mod engine;

#[derive(Clone)]
pub struct ServerState {
    pub engine_sender: Sender<(Request, oneshot::Sender<u64>)>,
}

pub async fn http_listen(state: ServerState, listen_address: &str) {
    let app = axum::Router::new()
        .route("/deposit", axum::routing::post(deposit))
        .route("/withdraw", axum::routing::post(withdraw))
        // .route("/orders", axum::routing::post(place_order))
        // .route("/cancel", axum::routing::post(cancel))
        // .route("/status", axum::routing::post(order_status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn deposit(
    ExtractState(state): ExtractState<ServerState>,
    Json(req): Json<DepositRequest>,
) -> Json<DepositResponse> {
    let sender = state.engine_sender;
    let (tx, rx) = oneshot::channel::<u64>();

    sender.send((Request::Deposit(req), tx)).await.expect("todo");
    let global_index = rx.await.expect("todo");
    println!("deposit: global_index: {:?}", global_index);
    // TODO if we want preconfs sign global index and put in response

    Json(DepositResponse { success: true })
}

async fn withdraw(
    ExtractState(state): ExtractState<ServerState>,
    Json(req): Json<WithdrawRequest>,
) -> Json<WithdrawResponse> {
    let sender = state.engine_sender;
    let (tx, rx) = oneshot::channel::<u64>();

    sender.send((Request::Withdraw(req), tx)).await.expect("todo");
    let global_index = rx.await.expect("todo");
    println!("withdraw: global_index: {:?}", global_index);
    // TODO if we want preconfs sign global index and put in response

    Json(WithdrawResponse { success: true })
}
