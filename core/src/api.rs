//! Types in the public API

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// All possible requests that can go into the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    /// [`AddOrderRequest`]
    AddOrder(AddOrderRequest),
    /// [`CancelOrderRequest`]
    CancelOrder(CancelOrderRequest),
    /// [`DepositRequest`]
    Deposit(DepositRequest),
    /// [`WithdrawRequest`]
    Withdraw(WithdrawRequest),
}

/// All possible responses from the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    /// [`AddOrderResponse`]
    AddOrder(AddOrderResponse),
    /// [`AddOrderResponse`]
    CancelOrder(CancelOrderResponse),
    /// [`DepositResponse`]
    Deposit(DepositResponse),
    /// [`WithdrawResponse`]
    Withdraw(WithdrawResponse),
}

/// A response from the clob engine with the global index of the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    /// The response from processing the request with one engine tick
    pub response: Response,
    /// The global index of the request. The request is guranteed to be processed
    /// via ordering indicated by this index
    pub global_index: u64,
}

/// Add a limit order.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderRequest {
    /// Account placing the order.
    pub address: [u8; 20],
    /// If this is a buy or sell order
    pub is_buy: bool,
    /// The price to execute the order at
    pub limit_price: u64,
    /// The size of the asset
    pub size: u64,
}

impl AddOrderRequest {
    /// Convert the order request to an [Order].
    pub const fn to_order(&self, oid: u64) -> Order {
        Order { is_buy: self.is_buy, limit_price: self.limit_price, size: self.size, oid }
    }
}

/// Response to [`AddOrderRequest`]
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderResponse {
    /// If the request was fully processed.
    pub success: bool,
    /// Any fills that happened when placing the order.
    pub status: Option<FillStatus>,
    // TODO: OID
}

/// Cancel a limit order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    /// Order ID.
    pub oid: u64,
}

/// Response to [`CancelOrderRequest`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderResponse {
    /// If the request was fully processed.
    pub success: bool,
    /// Any fills from the cancelled ordered that have already occurred.
    pub fill_status: Option<FillStatus>,
}

/// Deposit funds that can be use to place orders.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct DepositRequest {
    /// Account to credit funds to.
    pub address: [u8; 20],
    /// Amount of base asset to credit.
    pub base: AssetBalance,
}

/// Response to [`DepositRequest`]
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct DepositResponse {
    /// If the request was fully processed.
    pub success: bool,
}

/// Withdraw non locked funds.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    /// Account to debit funds from
    pub address: [u8; 20],
    /// Amount to withdraw from base asset free balance
    pub base_free: u64,
}

/// Response to [`WithdrawRequest`].
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawResponse {
    /// If the request was fully processed.
    pub success: bool,
}

/// All balances for a user.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct UserBalance {
    /// Users funds for selling
    pub a: u64,
    /// Users funds for buying
    pub b: u64,
    // TODO: do we need a third for funds that are not in limit orders
}

/// All balances for a user.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalance {
    /// Funds that are in the CLOB state, but not committed to orders.
    ///
    /// These funds can either be `lock`ed by a limit order or exit clob state
    /// by being moved to withdraw.
    ///
    /// To increase `free` funds, either some order needs to be filled or the
    /// user deposits into the clob.
    pub free: u64,
    /// Funds that are locked in orders. These funds can be
    pub locked: u64,
}

/// A limit order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// If the order is buy or sell.
    pub is_buy: bool,
    /// The price to execute the order at.
    pub limit_price: u64,
    /// Size of the asset to exchange.
    pub size: u64,
    /// Order ID.
    pub oid: u64,
}

impl Order {
    /// Create a new order
    pub const fn new(is_buy: bool, limit_price: u64, size: u64, oid: u64) -> Self {
        Self { is_buy, limit_price, size, oid }
    }
}

/// That current status of how filled an order is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct FillStatus {
    /// Order ID
    pub oid: u64,
    /// Size of the order
    pub size: u64,
    /// Account that owns the order
    pub address: [u8; 20],
    /// The amount of the order that has been filled.
    pub filled_size: u64,
    /// Each fill that has been executed.
    pub fills: Vec<OrderFill>,
}

/// A match of two orders.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderFill {
    /// Maker order ID.
    pub maker_oid: u64,
    /// Taker order ID.
    pub taker_oid: u64,
    /// Size the match.
    pub size: u64,
}

impl OrderFill {
    /// Create a new [Self].
    pub const fn new(maker_oid: u64, taker_oid: u64, size: u64) -> Self {
        Self { maker_oid, taker_oid, size }
    }
}
