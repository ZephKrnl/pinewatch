//! Every integer width as Unity's Mono compiles it: corefx `SByte`, `Byte`, `Int16` and `UInt16` narrow
//! `Number.TryParseInt32`; `Int64` and `UInt64` use corert's 64-bit paths.

use pinewatch_mono::{
    ExceptionKind, NetString, NumberFormat, NumberStyles, try_parse_i8, try_parse_i16, try_parse_i64, try_parse_u8,
    try_parse_u16, try_parse_u64,
};

macro_rules! parse {
    ($f:ident, $text:expr, $style:expr) => {
        $f(Some(&NetString::from_str($text)), $style, &NumberFormat::invariant()).expect("valid style")
    };
}

#[test]
fn widths_accept_their_range_only() {
    let any = NumberStyles::ANY;
    assert_eq!(parse!(try_parse_i8, "-128", any), Some(-128));
    assert_eq!(parse!(try_parse_i8, "127", any), Some(127));
    assert_eq!(parse!(try_parse_i8, "128", any), None);
    assert_eq!(parse!(try_parse_u8, "255", any), Some(255));
    assert_eq!(parse!(try_parse_u8, "256", any), None);
    assert_eq!(parse!(try_parse_u8, "-0", any), Some(0));
    assert_eq!(parse!(try_parse_u8, "-1", any), None);
    assert_eq!(parse!(try_parse_i16, "32768", any), None);
    assert_eq!(parse!(try_parse_u16, "65535", any), Some(65535));
    assert_eq!(parse!(try_parse_i64, "-9223372036854775808", any), Some(i64::MIN));
    assert_eq!(parse!(try_parse_i64, "9223372036854775808", any), None);
    assert_eq!(parse!(try_parse_u64, "18446744073709551615", any), Some(u64::MAX));
    assert_eq!(parse!(try_parse_u64, "18446744073709551616", any), None);
}

#[test]
fn hex_narrowing_follows_corefx() {
    let hex = NumberStyles::HEX_NUMBER;
    assert_eq!(parse!(try_parse_i8, "80", hex), Some(-128));
    assert_eq!(parse!(try_parse_i8, "100", hex), None);
    assert_eq!(parse!(try_parse_u8, "FF", hex), Some(255));
    assert_eq!(parse!(try_parse_i16, "8000", hex), Some(-32768));
    assert_eq!(parse!(try_parse_u16, "10000", hex), None);
    assert_eq!(parse!(try_parse_i64, "FFFFFFFFFFFFFFFF", hex), Some(-1));
    assert_eq!(parse!(try_parse_u64, "10000000000000000", hex), None);
}

#[test]
fn integer_style_fast_paths_for_64_bits() {
    let integer = NumberStyles::INTEGER;
    assert_eq!(parse!(try_parse_i64, " -9223372036854775808 ", integer), Some(i64::MIN));
    assert_eq!(parse!(try_parse_i64, "9223372036854775807", integer), Some(i64::MAX));
    assert_eq!(parse!(try_parse_i64, "9223372036854775808", integer), None);
    assert_eq!(parse!(try_parse_i64, "00000000000000000000000001", integer), Some(1));
    assert_eq!(parse!(try_parse_u64, "18446744073709551615", integer), Some(u64::MAX));
    assert_eq!(parse!(try_parse_u64, "18446744073709551616", integer), None);
    assert_eq!(parse!(try_parse_u64, "-0", integer), Some(0));
    assert_eq!(parse!(try_parse_u64, "-1", integer), None);
    assert_eq!(parse!(try_parse_u64, "5\0", integer), Some(5));
}

#[test]
fn every_width_validates_the_style() {
    let undefined = NumberStyles::from_bits(0x400);
    let invariant = NumberFormat::invariant();
    let kinds = [
        try_parse_i8(None, undefined, &invariant).unwrap_err().kind,
        try_parse_u8(None, undefined, &invariant).unwrap_err().kind,
        try_parse_i16(None, undefined, &invariant).unwrap_err().kind,
        try_parse_u16(None, undefined, &invariant).unwrap_err().kind,
        try_parse_i64(None, undefined, &invariant).unwrap_err().kind,
        try_parse_u64(None, undefined, &invariant).unwrap_err().kind,
    ];
    assert!(kinds.iter().all(|&kind| kind == ExceptionKind::Argument), "{kinds:?}");
}
