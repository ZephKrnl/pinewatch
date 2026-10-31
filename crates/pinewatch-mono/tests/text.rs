//! `new StreamReader(stream)` defaults: UTF-8 with its preamble, plus byte order mark detection.

use pinewatch_mono::decode_stream_reader_default;

fn decode(bytes: &[u8]) -> Vec<u16> {
    decode_stream_reader_default(bytes).as_units().to_vec()
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[test]
fn plain_utf8() {
    assert_eq!(decode(b"Key Value"), units("Key Value"));
}

#[test]
fn empty_input() {
    assert_eq!(decode(b""), units(""));
}

#[test]
fn utf8_bom_is_removed() {
    assert_eq!(decode(&[0xEF, 0xBB, 0xBF, b'A']), units("A"));
}

#[test]
fn utf8_bom_stops_further_detection() {
    // After the UTF-8 preamble matches, no other byte order mark is detected.
    assert_eq!(
        decode(&[0xEF, 0xBB, 0xBF, 0xFF, 0xFE, b'A', 0x00]),
        units("\u{FFFD}\u{FFFD}A\0")
    );
}

#[test]
fn utf16_le_bom() {
    assert_eq!(decode(&[0xFF, 0xFE, b'A', 0x00, b'B', 0x00]), units("AB"));
}

#[test]
fn utf16_be_bom() {
    assert_eq!(decode(&[0xFE, 0xFF, 0x00, b'A']), units("A"));
}

#[test]
fn utf32_le_bom() {
    assert_eq!(decode(&[0xFF, 0xFE, 0x00, 0x00, b'A', 0x00, 0x00, 0x00]), units("A"));
}

#[test]
fn utf32_be_bom() {
    assert_eq!(decode(&[0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, b'A']), units("A"));
}

#[test]
fn ff_fe_with_only_three_bytes_is_utf16_with_odd_trailing_byte() {
    assert_eq!(decode(&[0xFF, 0xFE, b'A']), units("\u{FFFD}"));
}

#[test]
fn invalid_utf8_byte_becomes_replacement_character() {
    assert_eq!(decode(&[b'A', 0xFF, b'B']), units("A\u{FFFD}B"));
}

#[test]
fn truncated_utf8_sequence_is_one_replacement_character() {
    assert_eq!(decode(&[0xE2, 0x82]), units("\u{FFFD}"));
}

#[test]
fn partial_utf8_preamble_is_decoded_as_utf8() {
    assert_eq!(decode(&[0xEF, 0xBB]), units("\u{FFFD}"));
}

#[test]
fn non_bmp_character_becomes_surrogate_pair() {
    assert_eq!(decode("\u{1F600}".as_bytes()), vec![0xD83D, 0xDE00]);
}
