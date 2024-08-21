//! Core logic and types of the `InfinityVM` CLOB.
//!
//! Note that everything in here needs to be able to target the ZKVM architecture

use std::collections::HashMap;

use crate::api::AssetBalance;
use api::{
    AddOrderRequest, AddOrderResponse, CancelOrderRequest, CancelOrderResponse, DepositRequest,
    DepositResponse, Request, Response, UserBalance, WithdrawRequest, WithdrawResponse,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

pub mod api;
pub mod orderbook;

use crate::api::FillStatus;
use orderbook::OrderBook;

/// Errors for this crate.
#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum Error {
    /// An order could not be found
    OrderDoesNotExist,
}

/// Input to the STF. Expected to be the exact input given to the ZKVM program.
pub type StfInput = (Request, ClobState);
/// Output from the STF. Expected to be the exact output from the ZKVM program.
pub type StfOutput = (Response, ClobState);

/// The state of the universe for the CLOB.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct ClobState {
    oid: u64,
    balances: HashMap<[u8; 20], UserBalance>,
    balances2: HashMap<[u8; 20], AssetBalance>,
    book: OrderBook,
    // TODO: ensure we are wiping order status for filled orders
    order_status: HashMap<u64, FillStatus>,
}

impl ClobState {
    /// Get the oid.
    pub fn oid(&self) -> u64 {
        self.oid
    }
    /// Get the balances.
    pub fn balances(&self) -> &HashMap<[u8; 20], UserBalance> {
        &self.balances
    }
    /// Get the book
    pub fn book(&self) -> &OrderBook {
        &self.book
    }
    /// Get the order status.
    pub fn order_status(&self) -> &HashMap<u64, FillStatus> {
        &self.order_status
    }
}

/// Deposit user funds that can be used to place orders.
pub fn deposit(req: DepositRequest, mut state: ClobState) -> (DepositResponse, ClobState) {
    // TODO, handle case of address already existing
    state.balances2.insert(req.address, req.base);

    (DepositResponse { success: true }, state)
}

/// Withdraw non-locked funds
pub fn withdraw(req: WithdrawRequest, mut state: ClobState) -> (WithdrawResponse, ClobState) {
    let addr = req.address;
    let base_balance = state.balances2.get_mut(&addr).expect("TODO");
    if base_balance.free < req.base_free {
        (WithdrawResponse { success: false }, state)
    } else {
        base_balance.free -= req.base_free;
        (WithdrawResponse { success: true }, state)
    }
}

/// Cancel an order.
pub fn cancel_order(
    req: CancelOrderRequest,
    mut state: ClobState,
) -> (CancelOrderResponse, ClobState) {
    if matches!(state.book.cancel(req.oid), Ok(())) {
        let fill_status = state.order_status.remove(&req.oid);
        (CancelOrderResponse { success: true, fill_status }, state)
    } else {
        (CancelOrderResponse { success: false, fill_status: None }, state)
    }
}

/// Add an order.
pub fn add_order(req: AddOrderRequest, mut state: ClobState) -> (AddOrderResponse, ClobState) {
    let addr = req.address;
    let balance = state.balances.get_mut(&addr).unwrap();

    if (req.is_buy && balance.b < req.size) || (!req.is_buy && balance.a < req.size) {
        return (AddOrderResponse { success: false, status: None }, state);
    };

    let order = req.to_order(state.oid);
    let order_id = order.oid;
    state.oid += 1;

    let (remaining_amount, fills) = state.book.limit(order);

    let fill_size = req.size - remaining_amount;
    if req.is_buy {
        balance.b -= req.size;
        balance.a += fill_size;
    } else {
        balance.a -= req.size;
        balance.b += fill_size;
    }

    for fill in fills.iter().cloned() {
        let maker_order_status = state.order_status.get_mut(&fill.maker_oid).unwrap();
        maker_order_status.filled_size += fill.size;
        if req.is_buy {
            state.balances.get_mut(&maker_order_status.address).expect("todo").b += fill.size;
        } else {
            state.balances.get_mut(&maker_order_status.address).expect("todo").a += fill.size;
        }
        maker_order_status.fills.push(fill);
    }

    let fill_status = FillStatus {
        oid: order_id,
        size: req.size,
        filled_size: fill_size,
        fills,
        address: req.address,
    };
    state.order_status.insert(order_id, fill_status.clone());

    let resp = AddOrderResponse { success: true, status: Some(fill_status) };

    (resp, state)
}

/// A tick is will execute a single request against the CLOB state.
pub fn tick(request: Request, state: ClobState) -> Result<(Response, ClobState), Error> {
    match request {
        Request::AddOrder(req) => {
            let (resp, state) = add_order(req, state);
            Ok((Response::AddOrder(resp), state))
        }
        Request::CancelOrder(req) => {
            let (resp, state) = cancel_order(req, state);
            Ok((Response::CancelOrder(resp), state))
        }
        Request::Deposit(req) => {
            let (resp, state) = deposit(req, state);
            Ok((Response::Deposit(resp), state))
        }
        Request::Withdraw(req) => {
            let (resp, state) = withdraw(req, state);
            Ok((Response::Withdraw(resp), state))
        }
    }
}

/// Trait for the sha256 hash of a borsh serialized type
pub trait BorshSha256 {
    /// The sha256 hash of a borsh serialized type
    fn borsh_sha256(&self) -> [u8; 32];
}

// Blanket impl. for any type that implements borsh serialize.
impl<T: BorshSerialize> BorshSha256 for T {
    fn borsh_sha256(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        borsh::to_writer(&mut hasher, &self).expect("orderbook is serializable");
        let hash = hasher.finalize();
        hash.into()
    }
}
