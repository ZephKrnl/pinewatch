//! Text decoding as done by `new StreamReader(stream)` with its defaults:
//! UTF-8 (with the UTF-8 preamble), byte order mark detection on, 1024-byte buffer.
//!
//! Port of `StreamReader.ReadBuffer`, `IsPreamble` and `DetectEncoding` from Mono's reference source.
//! Detection only sees the first buffer, which for a whole file is its first 1024 bytes.

use crate::NetString;

const UTF8_PREAMBLE: [u8; 3] = [0xEF, 0xBB, 0xBF];
const DEFAULT_BUFFER_SIZE: usize = 1024;
const REPLACEMENT: u16 = 0xFFFD;

/// Decodes a whole file the way the game's `StreamReader` reads it.
pub fn decode_stream_reader_default(bytes: &[u8]) -> NetString {
    let first = &bytes[..bytes.len().min(DEFAULT_BUFFER_SIZE)];

    // IsPreamble: compare the bytes available so far with the UTF-8 preamble.
    let available = first.len().min(UTF8_PREAMBLE.len());
    if !first.is_empty() && first[..available] == UTF8_PREAMBLE[..available] {
        if first.len() >= UTF8_PREAMBLE.len() {
            // Full match: the preamble is dropped and no other byte order mark is looked for.
            return decode_utf8(&bytes[UTF8_PREAMBLE.len()..]);
        }
        // The stream ended inside the preamble: the buffered bytes are decoded as UTF-8.
        return decode_utf8(bytes);
    }

    // DetectEncoding: needs at least two bytes.
    if first.len() >= 2 {
        match (first[0], first[1]) {
            (0xFE, 0xFF) => return decode_utf16(&bytes[2..], true),
            (0xFF, 0xFE) => {
                if first.len() < 4 || first[2] != 0 || first[3] != 0 {
                    return decode_utf16(&bytes[2..], false);
                }
                return decode_utf32(&bytes[4..], false);
            }
            (0x00, 0x00) if first.len() >= 4 && first[2] == 0xFE && first[3] == 0xFF => {
                return decode_utf32(&bytes[4..], true);
            }
            _ => {}
        }
    }

    decode_utf8(bytes)
}

/// UTF-8 with replacement: each maximal invalid subpart becomes one U+FFFD.
fn decode_utf8(bytes: &[u8]) -> NetString {
    NetString::from_units(String::from_utf8_lossy(bytes).encode_utf16().collect())
}

/// UTF-16 with replacement: unpaired surrogates and an odd trailing byte become U+FFFD.
fn decode_utf16(bytes: &[u8], big_endian: bool) -> NetString {
    let pairs = bytes.chunks_exact(2);
    let odd_tail = !pairs.remainder().is_empty();
    let raw = pairs.map(|p| if big_endian { u16::from_be_bytes([p[0], p[1]]) } else { u16::from_le_bytes([p[0], p[1]]) });

    let mut units = Vec::with_capacity(bytes.len() / 2 + 1);
    for decoded in char::decode_utf16(raw) {
        match decoded {
            Ok(ch) => {
                let mut buf = [0u16; 2];
                units.extend_from_slice(ch.encode_utf16(&mut buf));
            }
            Err(_) => units.push(REPLACEMENT),
        }
    }
    if odd_tail {
        units.push(REPLACEMENT);
    }
    NetString::from_units(units)
}

/// UTF-32 with replacement: invalid code points and a partial trailing group become U+FFFD.
fn decode_utf32(bytes: &[u8], big_endian: bool) -> NetString {
    let groups = bytes.chunks_exact(4);
    let partial_tail = !groups.remainder().is_empty();

    let mut units = Vec::with_capacity(bytes.len() / 2 + 1);
    for g in groups {
        let value = if big_endian {
            u32::from_be_bytes([g[0], g[1], g[2], g[3]])
        } else {
            u32::from_le_bytes([g[0], g[1], g[2], g[3]])
        };
        match char::from_u32(value) {
            Some(ch) => {
                let mut buf = [0u16; 2];
                units.extend_from_slice(ch.encode_utf16(&mut buf));
            }
            None => units.push(REPLACEMENT),
        }
    }
    if partial_tail {
        units.push(REPLACEMENT);
    }
    NetString::from_units(units)
}
