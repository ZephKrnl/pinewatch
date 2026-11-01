//! Integer parsing as Unity's Mono compiles it: `Int32.TryParse`/`UInt32.TryParse` (corefx) over corert's
//! `Number.Parsing.cs`. The first two tests are the `TryParseInt32` / `TryParseUInt32` cases from
//! `reference/u3-sdk/Assets/Tests/UnturnedDat/DatValueTests.cs` (NumberStyles.Any, invariant culture).

use pinewatch_mono::{ExceptionKind, NEW_LINE, NetString, NumberFormat, NumberStyles, sr, try_parse_i32, try_parse_u32};

fn i32_with(s: &str, style: NumberStyles) -> Option<i32> {
    try_parse_i32(Some(&NetString::from_str(s)), style, &NumberFormat::invariant()).expect("valid style")
}

fn u32_with(s: &str, style: NumberStyles) -> Option<u32> {
    try_parse_u32(Some(&NetString::from_str(s)), style, &NumberFormat::invariant()).expect("valid style")
}

fn i32_any(s: &str) -> Option<i32> {
    i32_with(s, NumberStyles::ANY)
}

fn u32_any(s: &str) -> Option<u32> {
    u32_with(s, NumberStyles::ANY)
}

fn i32_integer(s: &str) -> Option<i32> {
    i32_with(s, NumberStyles::INTEGER)
}

#[test]
fn dat_value_tests_try_parse_int32() {
    assert_eq!(i32_any("0"), Some(0));
    assert_eq!(i32_any("-2,147,483,648"), Some(i32::MIN));
    assert_eq!(i32_any("-2,147,483,649"), None);
    assert_eq!(i32_any("1"), Some(1));
    assert_eq!(i32_any("2,147,483,647"), Some(i32::MAX));
    assert_eq!(i32_any("2,147,483,648"), None);
    assert_eq!(i32_any("x"), None);
}

#[test]
fn dat_value_tests_try_parse_uint32() {
    assert_eq!(u32_any("0"), Some(0));
    assert_eq!(u32_any("-1"), None);
    assert_eq!(u32_any("1"), Some(1));
    assert_eq!(u32_any("4,294,967,295"), Some(u32::MAX));
    assert_eq!(u32_any("4,294,967,296"), None);
    assert_eq!(u32_any("x"), None);
}

#[test]
fn number_styles_any_accepts_dotnet_forms() {
    assert_eq!(i32_any("1,000"), Some(1000));
    assert_eq!(i32_any("(5)"), Some(-5));
    assert_eq!(i32_any("5-"), Some(-5));
    assert_eq!(i32_any(" 7 "), Some(7));
    assert_eq!(i32_any("\t\r\n7\u{000B}\u{000C}"), Some(7));
    assert_eq!(i32_any("5.0"), Some(5));
    assert_eq!(i32_any("5.5"), None);
    assert_eq!(i32_any("1e3"), Some(1000));
    assert_eq!(i32_any("1.5e1"), Some(15));
    assert_eq!(i32_any("15e-1"), None);
    assert_eq!(i32_any("\u{00A4}5"), Some(5));
    assert_eq!(i32_any("2147483648"), None);
    assert_eq!(i32_any("  -5"), Some(-5));
}

#[test]
fn number_styles_any_rejects() {
    assert_eq!(i32_any(",5"), None);
    assert_eq!(i32_any("1e"), None);
    assert_eq!(i32_any("+-5"), None);
    assert_eq!(i32_any("- 5"), None);
    assert_eq!(i32_any("(-5)"), None);
    assert_eq!(i32_any("0x10"), None);
    assert_eq!(i32_any("\u{0661}"), None);
    assert_eq!(i32_any(""), None);
    assert_eq!(i32_any("\u{00A0}5"), None);
}

#[test]
fn trailing_nul_characters_are_accepted() {
    assert_eq!(i32_any("5\0\0"), Some(5));
    assert_eq!(i32_any("5\0x"), None);
    assert_eq!(i32_integer("5\0\0"), Some(5));
    assert_eq!(i32_integer("5 \0"), Some(5));
    assert_eq!(i32_integer("5\0 "), None);
}

#[test]
fn negative_zero_quirks() {
    // Sign is cleared when no non-zero digit was seen and no decimal point was parsed.
    assert_eq!(u32_any("-0"), Some(0));
    assert_eq!(u32_any("-0.0"), None);
    assert_eq!(i32_any("-0"), Some(0));
    // The integer-style fast path allows a negative sign only on zero.
    assert_eq!(u32_with("-0", NumberStyles::INTEGER), Some(0));
    assert_eq!(u32_with("-000", NumberStyles::INTEGER), Some(0));
    assert_eq!(u32_with("-1", NumberStyles::INTEGER), None);
}

#[test]
fn more_than_fifty_digits_still_counts_scale() {
    let fifty_one_zeros_then_one = format!("{}1", "0".repeat(51));
    assert_eq!(i32_any(&fifty_one_zeros_then_one), Some(1));
    assert_eq!(i32_any(&"1".repeat(51)), None);
    assert_eq!(i32_integer(&fifty_one_zeros_then_one), Some(1));
}

#[test]
fn missing_string_fails() {
    assert_eq!(try_parse_i32(None, NumberStyles::ANY, &NumberFormat::invariant()), Ok(None));
}

#[test]
fn number_styles_integer_is_stricter() {
    assert_eq!(i32_integer("1,000"), None);
    assert_eq!(i32_integer("5.0"), None);
    assert_eq!(i32_integer(" -5 "), Some(-5));
    assert_eq!(i32_integer("(5)"), None);
    assert_eq!(i32_integer("5-"), None);
    assert_eq!(i32_integer("-2147483648"), Some(i32::MIN));
    assert_eq!(i32_integer("2147483648"), None);
    assert_eq!(i32_integer("-2147483649"), None);
    assert_eq!(u32_with("4294967295", NumberStyles::INTEGER), Some(u32::MAX));
    assert_eq!(u32_with("4294967296", NumberStyles::INTEGER), None);
    assert_eq!(i32_integer("   "), None);
    assert_eq!(i32_integer("-"), None);
}

#[test]
fn hex_number_style() {
    let hex = NumberStyles::HEX_NUMBER;
    assert_eq!(i32_with("ff", hex), Some(255));
    assert_eq!(i32_with(" 7FFFFFFF ", hex), Some(i32::MAX));
    assert_eq!(i32_with("FFFFFFFF", hex), Some(-1));
    assert_eq!(i32_with("000000000000000001", hex), Some(1));
    assert_eq!(i32_with("100000000", hex), None);
    assert_eq!(i32_with("0x10", hex), None);
    assert_eq!(i32_with("-1", hex), None);
    assert_eq!(i32_with("", hex), None);
    assert_eq!(u32_with("ffffffff\0", hex), Some(u32::MAX));
    assert_eq!(u32_with("\u{FF21}", hex), None);
}

#[test]
fn invalid_styles_throw_before_the_null_check() {
    let invariant = NumberFormat::invariant();
    let undefined = try_parse_i32(None, NumberStyles::from_bits(0x400), &invariant).unwrap_err();
    assert_eq!(
        (undefined.kind, undefined.message),
        (ExceptionKind::Argument, format!("{}{NEW_LINE}Parameter name: style", sr::ARGUMENT_INVALID_NUMBER_STYLES))
    );
    let hex_with_sign = NumberStyles::HEX_NUMBER.union(NumberStyles::ALLOW_LEADING_SIGN);
    let invalid_hex = try_parse_u32(Some(&NetString::from_str("1")), hex_with_sign, &invariant).unwrap_err();
    assert_eq!((invalid_hex.kind, invalid_hex.message.as_str()), (ExceptionKind::Argument, sr::ARG_INVALID_HEX_STYLE));
}

#[test]
fn culture_signs_take_different_paths_by_style() {
    // A sign containing NUL: the state machine (NumberStyles.Any) compares up to the NUL, like a C string;
    // the integer-style fast path compares the whole string.
    let nul_sign = NumberFormat { negative_sign: NetString::from_units(vec![0x2D, 0x00, 0x78]), ..NumberFormat::invariant() };
    let parse = |s: &str, style| try_parse_i32(Some(&NetString::from_str(s)), style, &nul_sign).unwrap();
    assert_eq!(parse("-5", NumberStyles::ANY), Some(-5));
    assert_eq!(parse("-5", NumberStyles::INTEGER), None);

    // A non-ASCII minus sign goes through StartsWith in the fast path.
    let minus = NumberFormat { negative_sign: NetString::from_str("\u{2212}"), ..NumberFormat::invariant() };
    let parse = |s: &str, style| try_parse_i32(Some(&NetString::from_str(s)), style, &minus).unwrap();
    assert_eq!(parse("\u{2212}5", NumberStyles::INTEGER), Some(-5));
    assert_eq!(parse("-5", NumberStyles::INTEGER), None);
    assert_eq!(parse("\u{2212}5", NumberStyles::ANY), Some(-5));
}
