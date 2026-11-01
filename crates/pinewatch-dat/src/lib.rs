//! Reader for the original game's `.dat` and `.asset` files.
//!
//! Port of `UnturnedDat` (`Assets/Runtime/UnturnedDat/`, U3-SDK v3.26.3.12), which the game ships as
//! `UnturnedDat.dll`. Behaviors that neither that assembly's IL nor a ported NUnit case confirms are listed in
//! `docs/VERIFICATION.md`.

mod dump;
mod parser;
mod tokenizer;
mod tree;

pub use parser::DatParser;
pub use tokenizer::{DatToken, DatTokenType, DatTokenizer};
pub use tree::{DatNodeType, DatTree, DictRef, ListRef, NodeId, NodeRef, ValueRef};
