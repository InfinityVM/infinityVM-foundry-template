use clob_core::api::Request;
use reth_db::{
    Database, TableType, TableViewer,
};

use reth_db::tables;
use std::fmt;

reth_db::tables! {
    /// Store global index
    /// 0 => global index of latest seen
    /// 1 => global index of latest processed
    table GlobalIndexTable<Key = u32, Value = u64>;

    /// Requests table, keyed by global index
    table RequestTable<Key = u64, Value = Request>;
}
