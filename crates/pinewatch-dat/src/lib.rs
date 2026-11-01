//! Reader for the original game's `.dat` and `.asset` files.
//!
//! Port of `UnturnedDat` (`Assets/Runtime/UnturnedDat/`, U3-SDK v3.26.3.12), which the game ships as
//! `UnturnedDat.dll`. Behaviors that neither that assembly's IL nor a ported NUnit case confirms are listed in
//! `docs/VERIFICATION.md`.

mod comment;
mod dictionary_ex;
mod dump;
mod list_ex;
mod parser;
mod structs;
mod tokenizer;
mod tree;
mod types;
mod value_ex;

pub use comment::DatComment;
pub use parser::DatParser;
pub use structs::DatParseable;
pub use types::{TypeHandle, TypeResolver};
pub use tokenizer::{DatToken, DatTokenType, DatTokenizer};
pub use tree::{DatNodeType, DatTree, DictRef, ListRef, NodeId, NodeRef, ValueRef};
