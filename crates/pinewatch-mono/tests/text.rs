//! `new StreamReader(stream)` defaults, as corefx implements them: UTF-8 with its preamble, byte order mark
//! detection, and decoders that are never flushed (`docs/VERIFICATION.md`, V4 and V5).

use pinewatch_mono::decode_stream_reader_default;

fn decode(bytes: &[u8]) -> Vec<u16> {
    decode_stream_reader_default(bytes).as_units().to_vec()
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

const R: &str = "\u{FFFD}";

#[test]
fn plain_utf8() {
    assert_eq!(decode(b"Key Value"), units("Key Value"));
    assert_eq!(decode("é€😀".as_bytes()), units("é€😀"));
}

#[test]
fn empty_input() {
    assert_eq!(decode(b""), units(""));
}

#[test]
fn utf8_bom_is_removed() {
    assert_eq!(decode(&[0xEF, 0xBB, 0xBF, b'A']), units("A"));
    assert_eq!(decode(&[0xEF, 0xBB, 0xBF]), units(""));
}

#[test]
fn utf8_bom_stops_further_detection() {
    // After the UTF-8 preamble matches, no other byte order mark is detected.
    assert_eq!(decode(&[0xEF, 0xBB, 0xBF, 0xFF, 0xFE, b'A', 0x00]), units(&format!("{R}{R}A\0")));
}

#[test]
fn byte_order_marks_are_only_detected_at_the_start() {
    assert_eq!(decode(&[b'A', 0xEF, 0xBB, 0xBF]), units("A\u{FEFF}"));
    let mut late = vec![b'A'; 1024];
    late.extend_from_slice(&[0xFF, 0xFE]);
    assert_eq!(decode(&late), units(&format!("{}{R}{R}", "A".repeat(1024))));
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
fn incomplete_utf8_at_the_end_is_dropped() {
    // The decoder keeps a valid but unfinished sequence for the next buffer; StreamReader never flushes it.
    assert_eq!(decode(&[0xE2, 0x82]), units(""));
    assert_eq!(decode(&[b'A', 0xF0, 0x9F, 0x98]), units("A"));
    assert_eq!(decode(&[b'A', 0xC3]), units("A"));
    assert_eq!(decode(&[b'A', 0xE0, 0xA0]), units("A"));
}

#[test]
fn partial_utf8_preamble_is_dropped() {
    // The bytes are buffered while the preamble might still match, then decoded without a flush at the end.
    assert_eq!(decode(&[0xEF, 0xBB]), units(""));
    assert_eq!(decode(&[0xEF]), units(""));
    assert_eq!(decode(&[0xEF, 0xBB, b'A']), units(&format!("{R}A")));
}

#[test]
fn incomplete_utf16_at_the_end_is_dropped() {
    assert_eq!(decode(&[0xFF, 0xFE, b'A']), units(""));
    assert_eq!(decode(&[0xFF, 0xFE, b'A', 0x00, b'B']), units("A"));
    assert_eq!(decode(&[0xFF, 0xFE, 0x3D, 0xD8]), units(""));
    assert_eq!(decode(&[0xFE, 0xFF, 0xD8, 0x3D, 0x00]), units(""));
}

#[test]
fn incomplete_utf32_at_the_end_is_dropped() {
    assert_eq!(decode(&[0xFF, 0xFE, 0x00, 0x00, b'A', 0x00, 0x00, 0x00, b'B', 0x00]), units("A"));
}

#[test]
fn invalid_utf8_bytes_are_replaced_one_per_fallback() {
    assert_eq!(decode(&[b'A', 0xFF, b'B']), units(&format!("A{R}B")));
    assert_eq!(decode(&[0x80]), units(R));
    assert_eq!(decode(&[0xC0, 0x80]), units(&format!("{R}{R}")));
    assert_eq!(decode(&[0xF5, 0x80]), units(&format!("{R}{R}")));
    // An interrupted sequence is one fallback; the interrupting byte is read again.
    assert_eq!(decode(&[0xE2, 0x82, b'A']), units(&format!("{R}A")));
    assert_eq!(decode(&[0xF0, 0x9F, 0x98, b'A']), units(&format!("{R}A")));
    assert_eq!(decode(&[0xE2, 0xE2, 0x82, 0xAC]), units(&format!("{R}€")));
}

#[test]
fn bad_second_byte_falls_back_together_with_its_lead_byte() {
    // Overlong forms, encoded surrogates and code points above U+10FFFF are caught at the second byte, and those
    // two bytes are one fallback. (Rust's from_utf8_lossy would replace each byte separately.)
    assert_eq!(decode(&[0xE0, 0x80, 0x80]), units(&format!("{R}{R}")));
    assert_eq!(decode(&[0xED, 0xA0, 0x80]), units(&format!("{R}{R}")));
    assert_eq!(decode(&[0xF0, 0x80, 0x80, 0x80]), units(&format!("{R}{R}{R}")));
    assert_eq!(decode(&[0xF4, 0x90, 0x80, 0x80]), units(&format!("{R}{R}{R}")));
    assert_eq!(decode(&[0xF4, 0x8F, 0xBF, 0xBF]), vec![0xDBFF, 0xDFFF]);
}

#[test]
fn invalid_utf16_units_are_replaced_one_per_fallback() {
    // Lone low surrogate; high surrogate followed by a normal character; two highs before a low.
    assert_eq!(decode(&[0xFF, 0xFE, 0x00, 0xDC, b'A', 0x00]), units(&format!("{R}A")));
    assert_eq!(decode(&[0xFF, 0xFE, 0x3D, 0xD8, b'A', 0x00]), units(&format!("{R}A")));
    let mut expected = units(R);
    expected.extend([0xD83D, 0xDE00]);
    assert_eq!(decode(&[0xFF, 0xFE, 0x3D, 0xD8, 0x3D, 0xD8, 0x00, 0xDE]), expected);
}

#[test]
fn invalid_utf32_values_are_replaced() {
    let mut bytes = vec![0xFF, 0xFE, 0x00, 0x00];
    bytes.extend_from_slice(&[0x00, 0xD8, 0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0xF6, 0x01, 0x00]);
    let mut expected = units(&format!("{R}{R}"));
    expected.extend([0xD83D, 0xDE00]);
    assert_eq!(decode(&bytes), expected);
}

#[test]
fn sequences_spanning_the_1024_byte_buffer_decode_whole() {
    let mut bytes = vec![b'A'; 1023];
    bytes.extend_from_slice("€".as_bytes());
    assert_eq!(decode(&bytes), units(&format!("{}€", "A".repeat(1023))));
}

#[test]
fn non_bmp_character_becomes_surrogate_pair() {
    assert_eq!(decode("\u{1F600}".as_bytes()), vec![0xD83D, 0xDE00]);
}
