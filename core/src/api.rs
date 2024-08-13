//! Types in the public API

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    AddOrder(AddOrderRequest),
    CancelOrder(CancelOrderRequest),
    Deposit(DepositRequest),
    Withdraw(WithdrawRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    AddOrder(AddOrderResponse),
    CancelOrder(CancelOrderResponse),
    Deposit(DepositResponse),
    Withdraw(WithdrawResponse),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderRequest {
    pub address: [u8; 20],
    pub is_buy: bool,
    pub limit_price: u64,
    pub size: u64,
}

impl AddOrderRequest {
    // TODO: add signing
    pub fn to_order(&self, oid: u64) -> Order {
        Order { is_buy: self.is_buy, limit_price: self.limit_price, size: self.size, oid }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOrderResponse {
    pub success: bool,
    pub status: Option<FillStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    pub oid: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderResponse {
    pub success: bool,
    pub fill_status: Option<FillStatus>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositRequest {
    pub address: [u8; 20],
    pub amounts: UserBalance,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositResponse {
    pub success: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    pub address: [u8; 20],
    pub amounts: UserBalance,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawResponse {
    pub success: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBalance {
    // Users funds that can be sold
    pub a: u64,
    // Users funds for buying
    pub b: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub is_buy: bool,
    pub limit_price: u64,
    pub size: u64,
    pub oid: u64,
}

impl Order {
    pub fn new(is_buy: bool, limit_price: u64, size: u64, oid: u64) -> Self {
        Self { is_buy, limit_price, size, oid }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct FillStatus {
    pub oid: u64,
    pub size: u64,
    pub address: [u8; 20],
    pub filled_size: u64,
    pub fills: Vec<OrderFill>,
}

#[derive(Deserialize, Serialize, Debug, Clone, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderFill {
    pub maker_oid: u64,
    pub taker_oid: u64,
    pub size: u64,
}

impl OrderFill {
    pub fn new(maker_oid: u64, taker_oid: u64, size: u64) -> Self {
        Self { maker_oid, taker_oid, size }
    }
}
