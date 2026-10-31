//! Integer parsing with Mono's `Number.ParseNumber` rules.
//! The first two tests are the `TryParseInt32` / `TryParseUInt32` cases from
//! `reference/u3-sdk/Assets/Tests/UnturnedDat/DatValueTests.cs` (NumberStyles.Any, invariant culture).

use pinewatch_mono::{NetString, NumberFormat, NumberStyles, try_parse_i32, try_parse_u32};

fn i32_any(s: &str) -> Option<i32> {
    try_parse_i32(Some(&NetString::from_str(s)), NumberStyles::ANY, &NumberFormat::invariant())
}

fn u32_any(s: &str) -> Option<u32> {
    try_parse_u32(Some(&NetString::from_str(s)), NumberStyles::ANY, &NumberFormat::invariant())
}

fn i32_integer(s: &str) -> Option<i32> {
    try_parse_i32(Some(&NetString::from_str(s)), NumberStyles::INTEGER, &NumberFormat::invariant())
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
}

#[test]
fn negative_zero_quirks() {
    // Sign is cleared when no non-zero digit was seen and no decimal point was parsed.
    assert_eq!(u32_any("-0"), Some(0));
    assert_eq!(u32_any("-0.0"), None);
    assert_eq!(i32_any("-0"), Some(0));
}

#[test]
fn more_than_fifty_digits_still_counts_scale() {
    let fifty_one_zeros_then_one = format!("{}1", "0".repeat(51));
    assert_eq!(i32_any(&fifty_one_zeros_then_one), Some(1));
    assert_eq!(i32_any(&"1".repeat(51)), None);
}

#[test]
fn missing_string_fails() {
    assert_eq!(try_parse_i32(None, NumberStyles::ANY, &NumberFormat::invariant()), None);
}

#[test]
fn number_styles_integer_is_stricter() {
    assert_eq!(i32_integer("1,000"), None);
    assert_eq!(i32_integer("5.0"), None);
    assert_eq!(i32_integer(" -5 "), Some(-5));
    assert_eq!(i32_integer("(5)"), None);
    assert_eq!(i32_integer("5-"), None);
}
