//! Mono class-library behavior the original game depends on (Unity 2022.3.62f3's Mono).
//!
//! Each item ports the corresponding Mono source (`mcs/class/referencesource/mscorlib` and
//! `mcs/class/corlib/ReferenceSources` in Unity's Mono fork). Behaviors that the Unity reference harness
//! has not confirmed yet are listed in `docs/VERIFICATION.md`.

pub mod chars;
mod exception;
mod guid;
mod net_string;
mod number;
mod parse_numbers;
mod text;

pub use exception::{ExceptionKind, MonoException};
pub use guid::Guid;
pub use net_string::NetString;
pub use number::{NumberFormat, NumberStyles, try_parse_i32, try_parse_u32};
pub use text::decode_stream_reader_default;
