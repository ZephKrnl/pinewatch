//! Reader for the KeyValueTable text format, used by `Level.hierarchy`.
//!
//! Port of `KeyValueTableReader` and the type readers in
//! `Framework/IO/FormattedFiles/KeyValueTables/` (U3-SDK v3.26.3.12).

mod reader;
mod values;

pub use reader::{KvtTable, KvtValue, read_kvt, read_kvt_bytes};
pub use values::KvtCursor;
