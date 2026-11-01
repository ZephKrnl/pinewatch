//! `new Guid(string)` as Unity's Mono compiles it: corefx `Guid.cs` and `ParseNumbers.cs`, with the English
//! resource strings (`docs/VERIFICATION.md`, V0).

use pinewatch_mono::{ExceptionKind, Guid, MonoException, NetString, sr};

const N: &str = "53d3ea4a555c44d1824b495c2b337aec";
const BYTES: [u8; 16] = [
    0x4a, 0xea, 0xd3, 0x53, 0x5c, 0x55, 0xd1, 0x44, 0x82, 0x4b, 0x49, 0x5c, 0x2b, 0x33, 0x7a, 0xec,
];

fn parse(s: &str) -> Result<Guid, MonoException> {
    Guid::parse(&NetString::from_str(s))
}

fn parse_err(s: &str) -> (ExceptionKind, String) {
    let e = parse(s).expect_err("throws");
    (e.kind, e.message)
}

fn format(message: &str) -> (ExceptionKind, String) {
    (ExceptionKind::Format, message.to_string())
}

/// `{0x<a>,0x<b>,0x<c>,{0x<d0>,...,0x<d7>}}` with every field 0 except the ones given.
fn x_form(a: &str, d0: &str) -> String {
    format!("{{0x{a},0x0,0x0,{{0x{d0},0x0,0x0,0x0,0x0,0x0,0x0,0x0}}}}")
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
fn length_errors() {
    assert_eq!(parse_err("0"), format(sr::FORMAT_GUID_INV_LEN));
    assert_eq!(parse_err("{53d3ea4a-555c-44d1-824b-495c2b337aec)"), format(sr::FORMAT_GUID_INV_LEN));
    // The last group stops at the first non-hex character: 11 digits instead of 12.
    assert_eq!(parse_err("00000000-0000-0000-0000-00000000000g"), format(sr::FORMAT_GUID_INV_LEN));
}

#[test]
fn empty_is_unrecognized() {
    assert_eq!(parse_err("   "), format(sr::FORMAT_GUID_UNRECOGNIZED));
}

#[test]
fn non_hex_character_without_dashes() {
    assert_eq!(parse_err("x3d3ea4a555c44d1824b495c2b337aec"), format(sr::FORMAT_GUID_INVALID_CHAR));
}

#[test]
fn dashed_group_stops_at_a_non_hex_character() {
    // ParseNumbers stops at 'g' after 7 digits, so the group is one digit short.
    assert_eq!(parse_err("0000000g-0000-0000-0000-000000000000"), format(sr::FORMAT_GUID_INVALID_CHAR));
}

#[test]
fn dashed_group_without_any_digit() {
    assert_eq!(parse_err("g0000000-0000-0000-0000-000000000000"), format(sr::FORMAT_NO_PARSIBLE_DIGITS));
}

#[test]
fn misplaced_dash() {
    assert_eq!(parse_err("53d3ea4a5-55c-44d1-824b-495c2b337aec"), format(sr::FORMAT_GUID_DASHES));
}

#[test]
fn dashed_groups_accept_a_plus_sign_and_a_hex_prefix() {
    // ParseNumbers skips a leading '+' and "0x", and the group still ends at the expected position.
    assert_eq!(parse("+0000000-0000-0000-0000-000000000000").unwrap(), Guid::EMPTY);
    let prefixed = parse("0x123456-0000-0000-0000-000000000000").unwrap();
    assert_eq!(&prefixed.as_bytes()[..4], &[0x56, 0x34, 0x12, 0x00]);
    assert_eq!(parse("00000000-0000-0000-0000-0x0000000000").unwrap(), Guid::EMPTY);
}

#[test]
fn minus_sign_is_unrecognized_with_the_argument_exception_inside() {
    let e = parse("-0000000-0000-0000-0000-000000000000").unwrap_err();
    assert_eq!((e.kind, e.message.as_str()), (ExceptionKind::Format, sr::FORMAT_GUID_UNRECOGNIZED));
    let inner = e.inner.expect("inner exception");
    assert_eq!((inner.kind, inner.message.as_str()), (ExceptionKind::Argument, sr::ARG_CANNOT_HAVE_NEGATIVE_VALUE));
    assert!(inner.inner.is_none());

    let braced = parse("{-3d3ea4a-555c-44d1-824b-495c2b337aec}").unwrap_err();
    assert_eq!(braced.message, sr::FORMAT_GUID_UNRECOGNIZED);
    assert_eq!(braced.inner.unwrap().kind, ExceptionKind::Argument);
}

#[test]
fn x_format_trailing_junk_in_a_number() {
    assert_eq!(parse_err(&x_form("1g", "0")), format(sr::FORMAT_EXTRA_JUNK_AT_END));
}

#[test]
fn x_format_overflowing_number_is_an_overflow_exception() {
    let e = parse(&x_form("123456789", "0")).unwrap_err();
    assert_eq!((e.kind, e.message.as_str()), (ExceptionKind::Overflow, sr::OVERFLOW_UINT32));
    let e = parse("{0x153d3ea4a,0x555c,0x44d1,{0x82,0x4b,0x49,0x5c,0x2b,0x33,0x7a,0xec}}").unwrap_err();
    assert_eq!(e.kind, ExceptionKind::Overflow);
}

#[test]
fn x_format_byte_over_255_is_a_format_exception() {
    assert_eq!(parse_err(&x_form("0", "182")), format(sr::OVERFLOW_BYTE));
}

#[test]
fn x_format_structure_errors() {
    // The hex-prefix message keeps its "{0}": Guid passes the argument with ParseFailureKind.Format.
    assert_eq!(parse_err("{1x0,0x0,0x0,{0x0}}"), format(sr::FORMAT_GUID_HEX_PREFIX));
    assert_eq!(sr::FORMAT_GUID_HEX_PREFIX, "Expected hex 0x in '{0}'.");
    assert_eq!(parse_err("{0x1}"), format(sr::FORMAT_GUID_COMMA));
    assert_eq!(parse_err("{0x1,0x2,0x3,0x4}"), format(sr::FORMAT_GUID_BRACE));
    assert_eq!(parse_err("{0x1,0x2,0x3,{0x4,0x5,0x6,0x7,0x8,0x9,0xa,0xb"), format(sr::FORMAT_GUID_BRACE_AFTER_LAST_NUMBER));
    assert_eq!(parse_err("{0x1,0x2,0x3,{0x4,0x5,0x6,0x7,0x8,0x9,0xa,0xb}"), format(sr::FORMAT_GUID_END_BRACE));
    assert_eq!(parse_err("{0x1,0x2,0x3,{0x4,0x5,0x6,0x7,0x8,0x9,0xa,0xb}}0"), format(sr::FORMAT_EXTRA_JUNK_AT_END));
    // A brace without "0x" right after it.
    assert_eq!(parse_err("{,0x0}"), format(sr::FORMAT_GUID_HEX_PREFIX));
}
