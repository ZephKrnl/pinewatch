//! Mono class-library behavior the original game depends on (Unity 2022.3.62f3's Mono).
//!
//! Each item ports the source file that Unity's Windows `unityjit` corlib build compiles: corefx and corert files
//! at the commits Unity's Mono pins, plus a few reference-source and Mono files (`docs/VERIFICATION.md`, V0).
//! Behaviors that the Unity reference harness has not confirmed yet are listed in `docs/VERIFICATION.md`.

pub mod chars;
mod exception;
mod guid;
mod net_string;
mod number;
mod parse_numbers;
pub mod sr;
mod text;

pub use exception::{ExceptionKind, MonoException, NEW_LINE};
pub use guid::Guid;
pub use net_string::NetString;
pub use number::{
    NumberFormat, NumberStyles, try_parse_i8, try_parse_i16, try_parse_i32, try_parse_i64, try_parse_u8, try_parse_u16,
    try_parse_u32, try_parse_u64,
};
pub use text::decode_stream_reader_default;
