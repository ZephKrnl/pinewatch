// Ported from corefx `src/Common/src/CoreLib/System/IO/StreamReader.cs` and `Text/UTF8Encoding.cs`,
// `Text/UnicodeEncoding.cs`, `Text/UTF32Encoding.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! Text decoding as `new StreamReader(stream)` does it with its defaults: UTF-8 with its preamble, byte order mark
//! detection on, a 1024-byte buffer.
//!
//! `ReadBuffer` decodes each buffer with `Decoder.GetChars(bytes, index, count, chars, index)`, which never flushes.
//! A sequence that is still incomplete when the stream ends stays inside the decoder and is never output.
//! Invalid input goes through the encodings' `DecoderReplacementFallback("�")`, which outputs one U+FFFD per
//! fallback, however many bytes that fallback covers. The decoders keep their state between buffers, so decoding
//! the whole stream at once gives the same characters as decoding it 1024 bytes at a time.

use crate::NetString;

const UTF8_PREAMBLE: [u8; 3] = [0xEF, 0xBB, 0xBF];
const DEFAULT_BUFFER_SIZE: usize = 1024;
const REPLACEMENT: u16 = 0xFFFD;

/// Decodes a whole file the way the game's `StreamReader` reads it (`ReadToEnd` over `ReadBuffer`, `IsPreamble`
/// and `DetectEncoding`). Detection only looks at the first buffer, which for a file is its first 1024 bytes.
pub fn decode_stream_reader_default(bytes: &[u8]) -> NetString {
    let first = &bytes[..bytes.len().min(DEFAULT_BUFFER_SIZE)];

    // IsPreamble: compare the bytes available so far with the UTF-8 preamble.
    let available = first.len().min(UTF8_PREAMBLE.len());
    if !first.is_empty() && first[..available] == UTF8_PREAMBLE[..available] {
        if first.len() >= UTF8_PREAMBLE.len() {
            // Full match: the preamble is dropped and no other byte order mark is looked for.
            return decode_utf8(&bytes[UTF8_PREAMBLE.len()..]);
        }
        // The stream ended inside the preamble: the buffered bytes are decoded as UTF-8, without detection.
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

// The pending-sequence flags `UTF8Encoding` keeps in the high bits of `ch`. They are pre-shifted so that each
// trailing byte's `ch << 6` moves the next copy into place; FINAL_BYTE reaches bit 29 with the last byte.
const FINAL_BYTE: i32 = 1 << 29;
const SUPPLIMENTARY_SEQ: i32 = 1 << 28;
const THREE_BYTE_SEQ: i32 = 1 << 27;

/// `UTF8Encoding.InRange`.
fn in_range(ch: i32, start: i32, end: i32) -> bool {
    (ch.wrapping_sub(start) as u32) <= (end.wrapping_sub(start) as u32)
}

/// `UTF8Encoding.GetChars` with a decoder that is never flushed (the slow loop; the fast loop hands every
/// irregular sequence back to it).
fn decode_utf8(bytes: &[u8]) -> NetString {
    let mut out = Vec::with_capacity(bytes.len());
    let mut ch: i32 = 0;
    let mut p = 0usize;

    while p < bytes.len() {
        if ch != 0 {
            // A pending sequence: we are expecting trailing bytes like 10vvvvvv.
            let cha = i32::from(bytes[p]);
            p += 1;
            if cha & 0xC0 != 0x80 {
                // This can start another sequence: put it back and fall back the pending bytes.
                p -= 1;
                out.push(REPLACEMENT);
                ch = 0;
                continue;
            }

            ch = ch.wrapping_shl(6) | (cha & 0x3F);
            if ch & FINAL_BYTE == 0 {
                if ch & SUPPLIMENTARY_SEQ != 0 {
                    if ch & (FINAL_BYTE >> 6) != 0 {
                        // Third byte of four: nothing to check.
                        continue;
                    }
                    // Second byte of four: the non-shortest form and the range up to U+10FFFF at once.
                    if !in_range(ch & 0x1F0, 0x10, 0x100) {
                        out.push(REPLACEMENT);
                        ch = 0;
                    }
                } else if (ch & (0x1F << 5)) == 0 || (ch & (0xF800 >> 6)) == (0xD800 >> 6) {
                    // Second byte of three: the non-shortest form, or an individually encoded surrogate.
                    out.push(REPLACEMENT);
                    ch = 0;
                }
                continue;
            }

            // The sequence is complete.
            if (ch & (SUPPLIMENTARY_SEQ | 0x1F0000)) > SUPPLIMENTARY_SEQ {
                out.push((((ch >> 10) & 0x7FF) + (0xD800 - (0x10000 >> 10))) as u16);
                ch = (ch & 0x3FF) + 0xDC00;
            }
            out.push(ch as u16);
            ch = 0;
            continue;
        }

        ch = i32::from(bytes[p]);
        p += 1;
        if ch > 0x7F {
            // Bit 6 has to be set in a lead byte.
            if ch & 0x40 == 0 {
                out.push(REPLACEMENT);
                ch = 0;
                continue;
            }
            if ch & 0x20 != 0 {
                if ch & 0x10 != 0 {
                    // Four bytes: F0 to F4 only.
                    ch &= 0x0F;
                    if ch > 0x04 {
                        out.push(REPLACEMENT);
                        ch = 0;
                        continue;
                    }
                    ch |= (FINAL_BYTE >> (3 * 6))
                        | (1 << 30)
                        | (3 << (30 - 2 * 6))
                        | SUPPLIMENTARY_SEQ
                        | (SUPPLIMENTARY_SEQ >> 6)
                        | (SUPPLIMENTARY_SEQ >> (2 * 6))
                        | (SUPPLIMENTARY_SEQ >> (3 * 6));
                } else {
                    // Three bytes.
                    ch = (ch & 0x0F)
                        | ((FINAL_BYTE >> (2 * 6)) | (1 << 30) | THREE_BYTE_SEQ | (THREE_BYTE_SEQ >> 6) | (THREE_BYTE_SEQ >> (2 * 6)));
                }
            } else {
                // Two bytes; C0 and C1 would be the non-shortest form.
                ch &= 0x1F;
                if ch <= 1 {
                    out.push(REPLACEMENT);
                    ch = 0;
                    continue;
                }
                ch |= FINAL_BYTE >> 6;
            }
            continue;
        }

        out.push(ch as u16);
        ch = 0;
    }

    // Not flushed: a pending sequence (ch != 0) stays in the decoder.
    NetString::from_units(out)
}

/// `UnicodeEncoding.GetChars` with a decoder that is never flushed. An odd byte (`lastByte`) and a high surrogate
/// waiting for its low surrogate (`lastChar`) stay in the decoder at the end.
fn decode_utf16(bytes: &[u8], big_endian: bool) -> NetString {
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut last_byte: Option<u8> = None;
    let mut last_char: u16 = 0;

    for &byte in bytes {
        let Some(first) = last_byte.take() else {
            last_byte = Some(byte);
            continue;
        };
        let ch = if big_endian { u16::from_be_bytes([first, byte]) } else { u16::from_le_bytes([first, byte]) };

        if (0xD800..=0xDFFF).contains(&ch) {
            if ch <= 0xDBFF {
                // A high surrogate; a previous one without its low surrogate is a fallback.
                if last_char > 0 {
                    out.push(REPLACEMENT);
                }
                last_char = ch;
                continue;
            }
            // A low surrogate without a high surrogate before it.
            if last_char == 0 {
                out.push(REPLACEMENT);
                continue;
            }
            out.push(last_char);
            last_char = 0;
        } else if last_char > 0 {
            // A high surrogate followed by something other than a low surrogate.
            out.push(REPLACEMENT);
            last_char = 0;
        }
        out.push(ch);
    }

    NetString::from_units(out)
}

/// `UTF32Encoding.GetChars` with a decoder that is never flushed. A partial group of fewer than 4 bytes stays in
/// the decoder at the end. Values above U+10FFFF and surrogates are one fallback each.
fn decode_utf32(bytes: &[u8], big_endian: bool) -> NetString {
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let (groups, _partial_group) = bytes.as_chunks::<4>();
    for &group in groups {
        let value = if big_endian { u32::from_be_bytes(group) } else { u32::from_le_bytes(group) };
        if value > 0x10FFFF || (0xD800..=0xDFFF).contains(&value) {
            out.push(REPLACEMENT);
        } else if value >= 0x10000 {
            out.push(((value - 0x10000) / 0x400 + 0xD800) as u16);
            out.push(((value - 0x10000) % 0x400 + 0xDC00) as u16);
        } else {
            out.push(value as u16);
        }
    }
    NetString::from_units(out)
}
