//! Tables for the database.

use clob_core::{
    api::{Request, Response},
    ClobState,
};
use reth_db::{tables, TableType, TableViewer};
use std::fmt;

reth_db::tables! {
    /// Store global index
    /// 0 => global index of latest seen
    /// 1 => global index of latest fully processed
    table GlobalIndexTable<Key = u32, Value = u64>;

    /// Requests table, keyed by global index
    table RequestTable<Key = u64, Value = Request>;

    /// Responses table, keyed by global index
    table ResponseTable<Key = u64, Value = Response>;

    /// ClOB State table, keyed by global index
    table ClobStateTable<Key = u64, Value = ClobState>;
}
