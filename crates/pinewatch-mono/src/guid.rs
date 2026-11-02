// Ported from corefx `src/Common/src/CoreLib/System/Guid.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! `System.Guid`: storage in .NET's byte layout, the `new Guid(string)` constructor, and `ToString("N")`.
//!
//! `new Guid(string)` is `TryParseGuid` with `GuidStyles.Any` and `GuidParseThrowStyle.All`, so every failure
//! throws at once: `SetFailure` raises `GetGuidParseException`, and exceptions from `ParseNumbers` are rethrown.
//! Messages are the English resource strings ([`sr`]).

use crate::parse_numbers::{IS_TIGHT, NO_SPACE, string_to_int, string_to_long};
use crate::{ExceptionKind, MonoException, NetString, chars, sr};

/// A GUID stored exactly as .NET stores it in memory: `a` (i32), `b`, `c` (i16) little-endian, then 8 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid([u8; 16]);

impl Guid {
    /// `Guid.Empty`.
    pub const EMPTY: Guid = Guid([0; 16]);

    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    fn from_fields(a: i32, b: i16, c: i16, rest: [u8; 8]) -> Self {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&a.to_le_bytes());
        bytes[4..6].copy_from_slice(&b.to_le_bytes());
        bytes[6..8].copy_from_slice(&c.to_le_bytes());
        bytes[8..16].copy_from_slice(&rest);
        Self(bytes)
    }

    /// `ToString("N")`: 32 lowercase hexadecimal digits.
    pub fn to_string_n(&self) -> String {
        let b = &self.0;
        let a = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let bb = u16::from_le_bytes([b[4], b[5]]);
        let c = u16::from_le_bytes([b[6], b[7]]);
        let mut out = format!("{a:08x}{bb:04x}{c:04x}");
        for byte in &b[8..16] {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }

    /// `ToString()` and `ToString("D")`: 32 lowercase hexadecimal digits in groups of 8, 4, 4, 4 and 12.
    pub fn to_string_d(&self) -> String {
        let n = self.to_string_n();
        format!("{}-{}-{}-{}-{}", &n[0..8], &n[8..12], &n[12..16], &n[16..20], &n[20..32])
    }

    /// `new Guid(string g)` for a non-null `g`.
    pub fn parse(g: &NetString) -> Result<Guid, MonoException> {
        try_parse_guid(g.as_units())
    }

    /// `Guid.TryParse(string input, out Guid result)`: `TryParseGuid` with `GuidParseThrowStyle.None`. Every
    /// failure there returns right after recording it, and exceptions from `ParseNumbers` are caught, so it succeeds
    /// exactly when `new Guid(input)` does, with the same value. A missing string fails.
    pub fn try_parse(input: Option<&NetString>) -> Option<Guid> {
        try_parse_guid(input?.as_units()).ok()
    }
}

impl std::fmt::Display for Guid {
    /// `Guid.ToString()`, the `D` format.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_d())
    }
}

/// `GuidResult.SetFailure(ParseFailureKind.Format, message)`: `new FormatException(message)`.
fn failure(message: &str) -> MonoException {
    MonoException::format(message)
}

/// `ReadOnlySpan<char>.Trim()`: removes `char.IsWhiteSpace` characters from both ends.
fn trim(s: &[u16]) -> &[u16] {
    let start = s.iter().position(|&c| !chars::is_white_space(c)).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| !chars::is_white_space(c)).map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

/// The span indexer: `IndexOutOfRangeException` outside the span.
fn at(s: &[u16], i: i32) -> Result<u16, MonoException> {
    usize::try_from(i).ok().and_then(|i| s.get(i).copied()).ok_or_else(MonoException::index_out_of_range)
}

/// `Slice(start)`: `ArgumentOutOfRangeException()` when `start` is past the end.
fn slice_from(s: &[u16], start: i32) -> Result<&[u16], MonoException> {
    if start as u32 > s.len() as u32 {
        return Err(MonoException::argument_out_of_range_default());
    }
    Ok(&s[start as usize..])
}

/// `Slice(start, length)`.
fn slice(s: &[u16], start: i32, length: i32) -> Result<&[u16], MonoException> {
    if start as u32 > s.len() as u32 || length as u32 > (s.len() as u32).wrapping_sub(start as u32) {
        return Err(MonoException::argument_out_of_range_default());
    }
    Ok(&s[start as usize..(start + length) as usize])
}

/// `IndexOf(char)`: -1 when absent.
fn index_of(s: &[u16], value: u8) -> i32 {
    s.iter().position(|&c| c == u16::from(value)).map_or(-1, |i| i as i32)
}

/// `TryParseGuid(guidString, GuidStyles.Any, ref result)` with `GuidParseThrowStyle.All`.
fn try_parse_guid(guid_string: &[u16]) -> Result<Guid, MonoException> {
    let guid_string = trim(guid_string);
    if guid_string.is_empty() {
        return Err(failure(sr::FORMAT_GUID_UNRECOGNIZED));
    }

    // GuidStyles.Any allows dashes, braces and parentheses and requires none of them, so the checks for
    // disallowed or missing ones never fail.
    let dashes_exist = guid_string.contains(&u16::from(b'-'));
    let braces_exist = guid_string.contains(&u16::from(b'{'));

    let result = if dashes_exist {
        try_parse_guid_with_dashes(guid_string)
    } else if braces_exist {
        try_parse_guid_with_hex_prefix(guid_string)
    } else {
        try_parse_guid_with_no_style(guid_string)
    };

    // catch (IndexOutOfRangeException) and catch (ArgumentException), which includes its subclasses:
    // SetFailure(ParseFailureKind.FormatWithInnerException, Format_GuidUnrecognized, ..., ex).
    match result {
        Err(e) if e.kind == ExceptionKind::IndexOutOfRange || e.kind.is_argument_exception() => {
            Err(failure(sr::FORMAT_GUID_UNRECOGNIZED).with_inner(e))
        }
        other => other,
    }
}

/// `{0xdddddddd,0xdddd,0xdddd,{0xdd,0xdd,0xdd,0xdd,0xdd,0xdd,0xdd,0xdd}}`
fn try_parse_guid_with_hex_prefix(guid_string: &[u16]) -> Result<Guid, MonoException> {
    let guid_string = eat_all_whitespace(guid_string);
    let s = guid_string.as_slice();

    if s.is_empty() || s[0] != u16::from(b'{') {
        return Err(failure(sr::FORMAT_GUID_BRACE));
    }
    // The hex-prefix failures pass "{0xdddddddd, etc}" and similar as a format argument, but with
    // ParseFailureKind.Format, which ignores it.
    if !is_hex_prefix(s, 1)? {
        return Err(failure(sr::FORMAT_GUID_HEX_PREFIX));
    }

    let mut num_start: i32 = 3;
    let mut num_len = index_of(slice_from(s, num_start)?, b',');
    if num_len <= 0 {
        return Err(failure(sr::FORMAT_GUID_COMMA));
    }
    let a = guid_string_to_int(slice(s, num_start, num_len)?, &mut 0, -1, IS_TIGHT)?;

    if !is_hex_prefix(s, num_start + num_len + 1)? {
        return Err(failure(sr::FORMAT_GUID_HEX_PREFIX));
    }
    num_start = num_start + num_len + 3;
    num_len = index_of(slice_from(s, num_start)?, b',');
    if num_len <= 0 {
        return Err(failure(sr::FORMAT_GUID_COMMA));
    }
    let b = guid_string_to_int(slice(s, num_start, num_len)?, &mut 0, -1, IS_TIGHT)? as i16;

    if !is_hex_prefix(s, num_start + num_len + 1)? {
        return Err(failure(sr::FORMAT_GUID_HEX_PREFIX));
    }
    num_start = num_start + num_len + 3;
    num_len = index_of(slice_from(s, num_start)?, b',');
    if num_len <= 0 {
        return Err(failure(sr::FORMAT_GUID_COMMA));
    }
    let c = guid_string_to_int(slice(s, num_start, num_len)?, &mut 0, -1, IS_TIGHT)? as i16;

    if s.len() as i32 <= num_start + num_len + 1 || at(s, num_start + num_len + 1)? != u16::from(b'{') {
        return Err(failure(sr::FORMAT_GUID_BRACE));
    }

    num_len += 1;
    let mut bytes = [0u8; 8];
    for (i, byte) in bytes.iter_mut().enumerate() {
        if !is_hex_prefix(s, num_start + num_len + 1)? {
            return Err(failure(sr::FORMAT_GUID_HEX_PREFIX));
        }
        // +3 to get by ',0x' or '{0x' for the first one.
        num_start = num_start + num_len + 3;
        if i < 7 {
            num_len = index_of(slice_from(s, num_start)?, b',');
            if num_len <= 0 {
                return Err(failure(sr::FORMAT_GUID_COMMA));
            }
        } else {
            num_len = index_of(slice_from(s, num_start)?, b'}');
            if num_len <= 0 {
                return Err(failure(sr::FORMAT_GUID_BRACE_AFTER_LAST_NUMBER));
            }
        }
        let number = guid_string_to_int(slice(s, num_start, num_len)?, &mut 0, -1, IS_TIGHT)? as u32;
        if number > 255 {
            return Err(failure(sr::OVERFLOW_BYTE));
        }
        *byte = number as u8;
    }

    let last = num_start + num_len + 1;
    if last >= s.len() as i32 || at(s, last)? != u16::from(b'}') {
        return Err(failure(sr::FORMAT_GUID_END_BRACE));
    }
    if last != s.len() as i32 - 1 {
        return Err(failure(sr::FORMAT_EXTRA_JUNK_AT_END));
    }

    Ok(Guid::from_fields(a, b, c, bytes))
}

/// `dddddddddddddddddddddddddddddddd`
fn try_parse_guid_with_no_style(s: &[u16]) -> Result<Guid, MonoException> {
    if s.len() != 32 {
        return Err(failure(sr::FORMAT_GUID_INV_LEN));
    }
    for &ch in s {
        if (u16::from(b'0')..=u16::from(b'9')).contains(&ch) {
            continue;
        }
        let upper = chars::to_upper_invariant(ch);
        if (u16::from(b'A')..=u16::from(b'F')).contains(&upper) {
            continue;
        }
        return Err(failure(sr::FORMAT_GUID_INVALID_CHAR));
    }

    let a = guid_string_to_int(slice(s, 0, 8)?, &mut 0, -1, IS_TIGHT)?;
    let b = guid_string_to_int(slice(s, 8, 4)?, &mut 0, -1, IS_TIGHT)? as i16;
    let c = guid_string_to_int(slice(s, 12, 4)?, &mut 0, -1, IS_TIGHT)? as i16;
    let temp = guid_string_to_int(slice(s, 16, 4)?, &mut 0, -1, IS_TIGHT)?;
    let start_pos = 20;
    let mut current_pos = start_pos;
    let templ = guid_string_to_long(s, &mut current_pos, NO_SPACE)?;
    if current_pos - start_pos != 12 {
        return Err(failure(sr::FORMAT_GUID_INV_LEN));
    }
    Ok(Guid::from_fields(a, b, c, tail_bytes(temp, templ)))
}

/// `[{|(]dddddddd-dddd-dddd-dddd-dddddddddddd[}|)]`
fn try_parse_guid_with_dashes(s: &[u16]) -> Result<Guid, MonoException> {
    let len = s.len();
    let start_pos: i32 = if s[0] == u16::from(b'{') {
        if len != 38 || s[37] != u16::from(b'}') {
            return Err(failure(sr::FORMAT_GUID_INV_LEN));
        }
        1
    } else if s[0] == u16::from(b'(') {
        if len != 38 || s[37] != u16::from(b')') {
            return Err(failure(sr::FORMAT_GUID_INV_LEN));
        }
        1
    } else if len != 36 {
        return Err(failure(sr::FORMAT_GUID_INV_LEN));
    } else {
        0
    };

    let dash = u16::from(b'-');
    if [8, 13, 18, 23].iter().any(|&offset| s[(offset + start_pos) as usize] != dash) {
        return Err(failure(sr::FORMAT_GUID_DASHES));
    }

    let mut current_pos = start_pos;
    let a = guid_string_to_int(s, &mut current_pos, 8, NO_SPACE)?;
    current_pos += 1;
    let b = guid_string_to_int(s, &mut current_pos, 4, NO_SPACE)? as i16;
    current_pos += 1;
    let c = guid_string_to_int(s, &mut current_pos, 4, NO_SPACE)? as i16;
    current_pos += 1;
    let temp = guid_string_to_int(s, &mut current_pos, 4, NO_SPACE)?;
    current_pos += 1;
    let start_pos = current_pos;
    let templ = guid_string_to_long(s, &mut current_pos, NO_SPACE)?;
    if current_pos - start_pos != 12 {
        return Err(failure(sr::FORMAT_GUID_INV_LEN));
    }
    Ok(Guid::from_fields(a, b, c, tail_bytes(temp, templ)))
}

/// Bytes `_d` to `_k` from the 4-digit group (`temp`) and the 12-digit group (`templ`).
fn tail_bytes(temp: i32, templ: i64) -> [u8; 8] {
    let high = (templ >> 32) as i32;
    let low = templ as i32;
    [
        (temp >> 8) as u8,
        temp as u8,
        (high >> 8) as u8,
        high as u8,
        (low >> 24) as u8,
        (low >> 16) as u8,
        (low >> 8) as u8,
        low as u8,
    ]
}

/// `Guid.StringToInt(str, ref parsePos, requiredLength, flags, ...)`: `ParseNumbers.StringToInt` in base 16,
/// whose exceptions `GuidParseThrowStyle.All` rethrows, then the digit-count check.
fn guid_string_to_int(
    s: &[u16],
    parse_pos: &mut i32,
    required_length: i32,
    flags: i32,
) -> Result<i32, MonoException> {
    let curr_start = *parse_pos;
    let result = string_to_int(s, 16, flags, parse_pos)?;
    if required_length != -1 && *parse_pos - curr_start != required_length {
        return Err(failure(sr::FORMAT_GUID_INVALID_CHAR));
    }
    Ok(result)
}

/// `Guid.StringToLong(str, ref parsePos, flags, ...)`.
fn guid_string_to_long(s: &[u16], parse_pos: &mut i32, flags: i32) -> Result<i64, MonoException> {
    string_to_long(s, 16, flags, parse_pos)
}

/// `EatAllWhitespace`: removes every `char.IsWhiteSpace` character.
fn eat_all_whitespace(s: &[u16]) -> Vec<u16> {
    s.iter().copied().filter(|&c| !chars::is_white_space(c)).collect()
}

/// `IsHexPrefix(str, i)`.
fn is_hex_prefix(s: &[u16], i: i32) -> Result<bool, MonoException> {
    if i + 1 >= s.len() as i32 || at(s, i)? != u16::from(b'0') {
        return Ok(false);
    }
    let x = at(s, i + 1)?;
    Ok(x == u16::from(b'x') || chars::to_lower_invariant(x) == u16::from(b'x'))
}
