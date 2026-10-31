//! `new Guid(string)` as implemented by Mono's reference-source `Guid` and `ParseNumbers`.
//! Mono's `Environment.GetResourceString` returns the resource key, so format errors carry the key as message.

use pinewatch_mono::{ExceptionKind, Guid, NetString};

const N: &str = "53d3ea4a555c44d1824b495c2b337aec";
const BYTES: [u8; 16] = [
    0x4a, 0xea, 0xd3, 0x53, 0x5c, 0x55, 0xd1, 0x44, 0x82, 0x4b, 0x49, 0x5c, 0x2b, 0x33, 0x7a, 0xec,
];

fn parse(s: &str) -> Result<Guid, (ExceptionKind, String)> {
    Guid::parse(&NetString::from_str(s)).map_err(|e| (e.kind, e.message))
}

#[test]
fn n_format() {
    assert_eq!(parse(N).unwrap().as_bytes(), &BYTES);
}

#[test]
fn d_format() {
    assert_eq!(parse("53d3ea4a-555c-44d1-824b-495c2b337aec").unwrap().as_bytes(), &BYTES);
}

#[test]
fn b_format() {
    assert_eq!(parse("{53d3ea4a-555c-44d1-824b-495c2b337aec}").unwrap().as_bytes(), &BYTES);
}

#[test]
fn p_format() {
    assert_eq!(parse("(53d3ea4a-555c-44d1-824b-495c2b337aec)").unwrap().as_bytes(), &BYTES);
}

#[test]
fn x_format() {
    let x = "{0x53d3ea4a,0x555c,0x44d1,{0x82,0x4b,0x49,0x5c,0x2b,0x33,0x7a,0xec}}";
    assert_eq!(parse(x).unwrap().as_bytes(), &BYTES);
}

#[test]
fn x_format_ignores_inner_whitespace() {
    let x = "{ 0x53d3ea4a, 0x555c, 0x44d1, { 0x82, 0x4b, 0x49, 0x5c, 0x2b, 0x33, 0x7a, 0xec } }";
    assert_eq!(parse(x).unwrap().as_bytes(), &BYTES);
}

#[test]
fn uppercase_and_surrounding_whitespace() {
    assert_eq!(parse(" 53D3EA4A555C44D1824B495C2B337AEC \t").unwrap().as_bytes(), &BYTES);
}

#[test]
fn to_string_n_is_32_lowercase_hex_digits() {
    assert_eq!(parse(N).unwrap().to_string_n(), N);
    assert_eq!(Guid::EMPTY.to_string_n(), "00000000000000000000000000000000");
}

#[test]
fn zero_is_invalid_length() {
    assert_eq!(parse("0"), Err((ExceptionKind::Format, "Format_GuidInvLen".to_string())));
}

#[test]
fn empty_is_unrecognized() {
    assert_eq!(parse("   "), Err((ExceptionKind::Format, "Format_GuidUnrecognized".to_string())));
}

#[test]
fn non_hex_character_is_invalid_char() {
    assert_eq!(
        parse("x3d3ea4a555c44d1824b495c2b337aec"),
        Err((ExceptionKind::Format, "Format_GuidInvalidChar".to_string()))
    );
}

#[test]
fn misplaced_dash() {
    assert_eq!(
        parse("53d3ea4a5-55c-44d1-824b-495c2b337aec"),
        Err((ExceptionKind::Format, "Format_GuidDashes".to_string()))
    );
}

#[test]
fn byte_over_255_in_x_format() {
    let x = "{0x53d3ea4a,0x555c,0x44d1,{0x182,0x4b,0x49,0x5c,0x2b,0x33,0x7a,0xec}}";
    assert_eq!(parse(x), Err((ExceptionKind::Format, "Overflow_Byte".to_string())));
}

#[test]
fn overflowing_first_number_in_x_format_is_overflow_exception() {
    let x = "{0x153d3ea4a,0x555c,0x44d1,{0x82,0x4b,0x49,0x5c,0x2b,0x33,0x7a,0xec}}";
    assert_eq!(parse(x).unwrap_err().0, ExceptionKind::Overflow);
}

#[test]
fn minus_sign_in_dashed_form_is_unrecognized() {
    assert_eq!(
        parse("{-3d3ea4a-555c-44d1-824b-495c2b337aec}"),
        Err((ExceptionKind::Format, "Format_GuidUnrecognized".to_string()))
    );
}
