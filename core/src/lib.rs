//! Core logic and types of the `InfinityVM` CLOB.
//!
//! Note that everything in here needs to be able to target the ZKVM architecture

use std::collections::HashMap;

use crate::api::AssetBalance;
use api::{
    AddOrderRequest, AddOrderResponse, CancelOrderRequest, CancelOrderResponse, DepositRequest,
    DepositResponse, Request, Response, WithdrawRequest, WithdrawResponse,
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
    base_balances: HashMap<[u8; 20], AssetBalance>,
    quote_balances: HashMap<[u8; 20], AssetBalance>,
    book: OrderBook,
    // TODO: ensure we are wiping order status for filled orders
    order_status: HashMap<u64, FillStatus>,
}

impl ClobState {
    /// Get the oid.
    pub fn oid(&self) -> u64 {
        self.oid
    }
    /// Get the base asset balances.
    pub fn base_balances(&self) -> &HashMap<[u8; 20], AssetBalance> {
        &self.base_balances
    }
    /// Get the base asset balances.
    pub fn quote_balances(&self) -> &HashMap<[u8; 20], AssetBalance> {
        &self.quote_balances
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
    let base_balance = state.base_balances.entry(req.address).or_default();
    base_balance.free += req.base_free;

    let quote_balance = state.base_balances.entry(req.address).or_default();
    quote_balance.free += req.quote_free;

    (DepositResponse { success: true }, state)
}

/// Withdraw non-locked funds
pub fn withdraw(req: WithdrawRequest, mut state: ClobState) -> (WithdrawResponse, ClobState) {
    let addr = req.address;
    let base_balance = state.base_balances.get_mut(&addr).expect("TODO");
    let quote_balance = state.quote_balances.get_mut(&addr).expect("TODO");
    if base_balance.free < req.base_free || quote_balance.free < req.quote_free {
        (WithdrawResponse { success: false }, state)
    } else {
        base_balance.free -= req.base_free;
        quote_balance.free -= req.quote_free;
        (WithdrawResponse { success: true }, state)
    }
}

/// Cancel an order.
pub fn cancel_order(
    req: CancelOrderRequest,
    mut state: ClobState,
) -> (CancelOrderResponse, ClobState) {
    let order = match state.book.cancel(req.oid) {
        Ok(o) => o,
        Err(_) => return (CancelOrderResponse { success: false, fill_status: None }, state),
    };

    if order.is_buy {
        // TODO(now): when we add quote, make sure to credit free quote
        let quote_balances = state.quote_balances.get_mut(&order.address).expect("todo");
        let quote_size = order.quote_size();
        quote_balances.free += quote_size;
        quote_balances.locked -= quote_size;
    } else {
        let base_balance = state.base_balances.get_mut(&order.address).expect("todo");
        base_balance.free += order.size;
        base_balance.locked -= order.size
    };

    let fill_status = state.order_status.remove(&order.oid);
    (CancelOrderResponse { success: false, fill_status }, state)
}

/// Add an order.
pub fn add_order(req: AddOrderRequest, mut state: ClobState) -> (AddOrderResponse, ClobState) {
    let o = req.to_order(state.oid);
    state.oid += 1;

    let base_balance = state.base_balances.get(&o.address).unwrap();
    let quote_balance = state.quote_balances.get(&o.address).unwrap();

    let o = req.to_order(state.oid);
    let order_id = o.oid;
    state.oid += 1;

    let is_invalid_buy = o.is_buy && quote_balance.free < o.quote_size();
    let is_invalid_sell = !o.is_buy && base_balance.free < o.size;
    if is_invalid_buy || is_invalid_sell {
        return (AddOrderResponse { success: false, status: None }, state);
    };

    let (remaining_amount, fills) = state.book.limit(o);

    for fill in fills.iter().cloned() {
        let maker_order_status = state
            .order_status
            .get_mut(&fill.maker_oid)
            .expect("fill status is created when order is added");
        maker_order_status.filled_size += fill.size;

        if req.is_buy {
            // Seller exchanges base for quote
            state.base_balances.entry(fill.seller).and_modify(|b| b.locked -= fill.size);
            state.quote_balances.entry(fill.seller).and_modify(|b| b.free += fill.quote_size());

            // Buyer exchanges quote for base
            state.base_balances.entry(req.address).and_modify(|b| b.free += fill.size);
            state.quote_balances.entry(req.address).and_modify(|b| b.locked -= fill.quote_size());
        } else {
            state.base_balances.entry(req.address).and_modify(|b| b.locked -= fill.size);
            state.quote_balances.entry(req.address).and_modify(|b| b.free += fill.quote_size());

            // Buyer exchanges quote for base
            state.base_balances.entry(fill.buyer).and_modify(|b| b.free += fill.size);
            state.quote_balances.entry(fill.buyer).and_modify(|b| b.locked -= fill.quote_size());
        }
        maker_order_status.fills.push(fill);
    }

    let fill_size = req.size - remaining_amount;
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
