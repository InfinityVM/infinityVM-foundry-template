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

const DEPOSIT: &str = "/deposit";
const WITHDRAW: &str = "/withdraw";
const ORDERS: &str = "/orders";
const CANCEL: &str = "/cancel";
const CLOB_STATE: &str = "/clob-state";

/// Stateful parts of REST server
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Engine send channel handle.
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    /// The database
    db: Arc<DatabaseEnv>,
}

impl ServerState {
    /// Create a new instance of [Self].
    pub fn new(
        engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
        db: Arc<DatabaseEnv>,
    ) -> Self {
        Self { engine_sender, db }
    }
}

fn app(state: ServerState) -> Router {
    axum::Router::new()
        .route(DEPOSIT, axum::routing::post(deposit))
        .route(WITHDRAW, axum::routing::post(withdraw))
        .route(ORDERS, axum::routing::post(add_order))
        .route(CANCEL, axum::routing::post(cancel))
        .route(CLOB_STATE, axum::routing::get(clob_state))
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

#[cfg(test)]
mod tests {
    // ref for testing: https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs
    use super::*;
    use axum::{
        body::Body,
        extract::connect_info::MockConnectInfo,
        http::{self, Request as AxumRequest, StatusCode},
    };
    use clob_core::api::{DepositResponse, UserBalance};
    use http_body_util::BodyExt;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;
    use tokio::task::JoinHandle;
    use tower::{Service, ServiceExt};

    const CHANEL_SIZE: usize = 32;

    // Simple wrapper for tokio task handle to make sure it aborts
    struct DropAbort<T>(JoinHandle<T>);
    impl<T> Drop for DropAbort<T> {
        fn drop(&mut self) {
            self.0.abort_handle().abort();
        }
    }

    async fn test_setup() -> (ServerState, DropAbort<()>) {
        let dbdir = tempdir().unwrap();
        let db = Arc::new(crate::db::init_db(dbdir).unwrap());
        let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(CHANEL_SIZE);

        let server_state = ServerState::new(engine_sender, Arc::clone(&db));

        let engine_handle =
            tokio::spawn(async move { crate::engine::run_engine(engine_receiver, db).await });

        (server_state, DropAbort(engine_handle))
    }

    async fn post<Req, Resp>(app: &mut Router, uri: &str, req: Req) -> Resp
    where
        Req: serde::Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let body = Body::from(serde_json::to_vec(&req).unwrap());

        let request = AxumRequest::builder()
            .uri(uri)
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .method(http::Method::POST)
            .body(body)
            .unwrap();

        let response =
            ServiceExt::<AxumRequest<Body>>::ready(app).await.unwrap().call(request).await.unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn place_bids() {
        let (server_state, _) = test_setup().await;
        let mut app = app(server_state);

        let response: ApiResponse = post(
            &mut app,
            DEPOSIT,
            DepositRequest { address: [0; 20], amounts: UserBalance { a: 10, b: 10 } },
        )
        .await;
    }
}
