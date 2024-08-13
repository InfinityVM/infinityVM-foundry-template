use std::collections::HashMap;

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

#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum Error {
    OrderDoesNotExist,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct State {
    oid: u64,
    balances: HashMap<[u8; 20], UserBalance>,
    book: OrderBook,
    // TODO: ensure we are wiping order status for filled orders
    order_status: HashMap<u64, FillStatus>,
}

pub fn deposit(req: DepositRequest, mut state: State) -> (DepositResponse, State) {
    // TODO, handle case of address already existing
    state.balances.insert(req.address, req.amounts);

    (DepositResponse { success: true }, state)
}

pub fn withdraw(req: WithdrawRequest, mut state: State) -> (WithdrawResponse, State) {
    let addr = req.address;
    let balance = state.balances.get_mut(&addr).expect("TODO");
    if balance.a < req.amounts.a || balance.b < req.amounts.b {
        (WithdrawResponse { success: false }, state)
    } else {
        balance.a -= req.amounts.a;
        balance.b -= req.amounts.b;
        (WithdrawResponse { success: true }, state)
    }
}

pub fn cancel_order(req: CancelOrderRequest, mut state: State) -> (CancelOrderResponse, State) {
    if let Ok(()) = state.book.cancel(req.oid) {
        let fill_status = state.order_status.remove(&req.oid);
        (CancelOrderResponse { success: true, fill_status }, state)
    } else {
        (CancelOrderResponse { success: false, fill_status: None }, state)
    }
}

pub fn add_order(req: AddOrderRequest, mut state: State) -> (AddOrderResponse, State) {
    let addr = req.address;
    let balance = state.balances.get_mut(&addr).unwrap();

    // -- External
    if (req.is_buy && balance.b < req.size) || (!req.is_buy && balance.a < req.size) {
        return (AddOrderResponse { success: false, status: None }, state);
    };

    let order = req.to_order(state.oid);
    let order_id = order.oid;
    state.oid += 1;
    // --

    // --- Internal
    let (remaining_amount, fills) = state.book.limit(order);
    // ---

    // -- External
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
    // --

    let resp = AddOrderResponse { success: true, status: Some(fill_status) };

    (resp, state)
}

pub fn tick(request: Request, state: State) -> Result<(Response, State), Error> {
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
