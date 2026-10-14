//! Folio Sheet's calculation engine: cell addresses, a formula parser, a lazy evaluator that is
//! safe against cycles, and CSV in and out. No windows, no files.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod addr;
mod csv;
mod eval;
mod parse;
mod value;

pub use addr::{Addr, col_name, parse_addr};
pub use csv::{from_csv, to_csv};
pub use eval::Sheet;
pub use value::{ErrCode, Value};

/// Sheet limits: columns A..ZZ-ish are fine, but ranges are capped so a formula cannot hang us.
pub const MAX_RANGE_CELLS: usize = 200_000;
