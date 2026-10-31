//! `System.Guid`: storage in .NET's byte layout, the `new Guid(string)` constructor, and `ToString("N")`.
//!
//! Port of `Guid.TryParseGuid` with `GuidStyles.Any` and `GuidParseThrowStyle.All` from Mono's reference source.
//! Mono's `Environment.GetResourceString` returns the resource key, so format errors carry keys such as
//! `Format_GuidInvLen` as their message.

use crate::parse_numbers::{IS_TIGHT, NO_SPACE, string_to_int, string_to_long};
use crate::{ExceptionKind, MonoException, NetString, chars};

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

    /// `new Guid(string g)`.
    pub fn parse(g: &NetString) -> Result<Guid, MonoException> {
        let guid_string = trim(g.as_units());
        if guid_string.is_empty() {
            return Err(failure("Format_GuidUnrecognized"));
        }

        let has_dashes = guid_string.contains(&u16::from(b'-'));
        let has_braces = guid_string.contains(&u16::from(b'{'));
        let result = if has_dashes {
            parse_with_dashes(guid_string)
        } else if has_braces {
            parse_with_hex_prefix(guid_string)
        } else {
            parse_with_no_style(guid_string)
        };

        // TryParseGuid catches IndexOutOfRangeException and ArgumentException (with subclasses).
        match result {
            Err(e) if e.kind == ExceptionKind::IndexOutOfRange || e.kind.is_argument_exception() => {
                Err(failure("Format_GuidUnrecognized"))
            }
            other => other,
        }
    }
}

/// `GuidResult.SetFailure(ParseFailureKind.Format, key)` with `GuidParseThrowStyle.All`.
fn failure(key: &str) -> MonoException {
    MonoException::format(key)
}

/// `string.Trim()`.
fn trim(s: &[u16]) -> &[u16] {
    let start = s.iter().position(|&c| !chars::is_white_space(c)).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| !chars::is_white_space(c)).map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

/// `string.Substring(start, length)`.
fn substring(s: &[u16], start: i32, length: i32) -> Result<&[u16], MonoException> {
    if start < 0 {
        return Err(MonoException::argument_out_of_range("startIndex"));
    }
    if length < 0 {
        return Err(MonoException::argument_out_of_range("length"));
    }
    if start as usize > s.len() || (start as usize) + (length as usize) > s.len() {
        return Err(MonoException::argument_out_of_range(if start as usize > s.len() { "startIndex" } else { "length" }));
    }
    Ok(&s[start as usize..(start + length) as usize])
}

/// `string.IndexOf(char, startIndex)`: -1 when not found; throws when `startIndex` is past the end.
fn index_of(s: &[u16], value: u8, start: i32) -> Result<i32, MonoException> {
    if start < 0 || start as usize > s.len() {
        return Err(MonoException::argument_out_of_range("startIndex"));
    }
    Ok(s[start as usize..]
        .iter()
        .position(|&c| c == u16::from(value))
        .map_or(-1, |i| start + i as i32))
}

/// The C# string indexer.
fn char_at(s: &[u16], i: i32) -> Result<u16, MonoException> {
    usize::try_from(i).ok().and_then(|i| s.get(i).copied()).ok_or_else(MonoException::index_out_of_range)
}

/// `Guid.StringToInt` with a required length: `Format_GuidInvalidChar` when the digit count is wrong.
fn string_to_int_at(s: &[u16], pos: &mut i32, required_length: i32, flags: i32) -> Result<i32, MonoException> {
    let start = *pos;
    let result = string_to_int(Some(s), 16, flags, Some(pos))?;
    if required_length != -1 && *pos - start != required_length {
        return Err(failure("Format_GuidInvalidChar"));
    }
    Ok(result)
}

fn split_tail(temp: i32, templ: i64) -> [u8; 8] {
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

/// `dddddddddddddddddddddddddddddddd`
fn parse_with_no_style(s: &[u16]) -> Result<Guid, MonoException> {
    if s.len() != 32 {
        return Err(failure("Format_GuidInvLen"));
    }
    for &ch in s {
        if (0x30..=0x39).contains(&ch) {
            continue;
        }
        let upper = chars::to_upper_invariant(ch);
        if (0x41..=0x46).contains(&upper) {
            continue;
        }
        return Err(failure("Format_GuidInvalidChar"));
    }

    let a = string_to_int(Some(substring(s, 0, 8)?), 16, IS_TIGHT, None)?;
    let b = string_to_int(Some(substring(s, 8, 4)?), 16, IS_TIGHT, None)? as i16;
    let c = string_to_int(Some(substring(s, 12, 4)?), 16, IS_TIGHT, None)? as i16;
    let temp = string_to_int(Some(substring(s, 16, 4)?), 16, IS_TIGHT, None)?;
    let start = 20;
    let mut current = start;
    let templ = string_to_long(Some(s), 16, NO_SPACE, Some(&mut current))?;
    if current - start != 12 {
        return Err(failure("Format_GuidInvLen"));
    }
    Ok(Guid::from_fields(a, b, c, split_tail(temp, templ)))
}

/// `[{|(]dddddddd-dddd-dddd-dddd-dddddddddddd[}|)]`
fn parse_with_dashes(s: &[u16]) -> Result<Guid, MonoException> {
    let len = s.len();
    let start_pos: i32 = match s[0] {
        0x7B => {
            if len != 38 || s[37] != u16::from(b'}') {
                return Err(failure("Format_GuidInvLen"));
            }
            1
        }
        0x28 => {
            if len != 38 || s[37] != u16::from(b')') {
                return Err(failure("Format_GuidInvLen"));
            }
            1
        }
        _ => {
            if len != 36 {
                return Err(failure("Format_GuidInvLen"));
            }
            0
        }
    };

    let dash = u16::from(b'-');
    if [8, 13, 18, 23].iter().any(|&offset| s[(offset + start_pos) as usize] != dash) {
        return Err(failure("Format_GuidDashes"));
    }

    let mut current = start_pos;
    let a = string_to_int_at(s, &mut current, 8, NO_SPACE)?;
    current += 1;
    let b = string_to_int_at(s, &mut current, 4, NO_SPACE)? as i16;
    current += 1;
    let c = string_to_int_at(s, &mut current, 4, NO_SPACE)? as i16;
    current += 1;
    let temp = string_to_int_at(s, &mut current, 4, NO_SPACE)?;
    current += 1;
    let start = current;
    let templ = string_to_long(Some(s), 16, NO_SPACE, Some(&mut current))?;
    if current - start != 12 {
        return Err(failure("Format_GuidInvLen"));
    }
    Ok(Guid::from_fields(a, b, c, split_tail(temp, templ)))
}

/// `Guid.IsHexPrefix`.
fn is_hex_prefix(s: &[u16], i: i32) -> bool {
    i >= 0
        && s.len() as i32 > i + 1
        && s[i as usize] == u16::from(b'0')
        && chars::to_lower_invariant(s[(i + 1) as usize]) == u16::from(b'x')
}

/// `{0xdddddddd,0xdddd,0xdddd,{0xdd,0xdd,0xdd,0xdd,0xdd,0xdd,0xdd,0xdd}}`
fn parse_with_hex_prefix(s: &[u16]) -> Result<Guid, MonoException> {
    let s: Vec<u16> = s.iter().copied().filter(|&c| !chars::is_white_space(c)).collect();
    let s = s.as_slice();

    if s.is_empty() || s[0] != u16::from(b'{') {
        return Err(failure("Format_GuidBrace"));
    }
    if !is_hex_prefix(s, 1) {
        return Err(failure("Format_GuidHexPrefix"));
    }

    let mut num_start: i32 = 3;
    let mut num_len = index_of(s, b',', num_start)? - num_start;
    if num_len <= 0 {
        return Err(failure("Format_GuidComma"));
    }
    let a = string_to_int(Some(substring(s, num_start, num_len)?), 16, IS_TIGHT, None)?;

    if !is_hex_prefix(s, num_start + num_len + 1) {
        return Err(failure("Format_GuidHexPrefix"));
    }
    num_start = num_start + num_len + 3;
    num_len = index_of(s, b',', num_start)? - num_start;
    if num_len <= 0 {
        return Err(failure("Format_GuidComma"));
    }
    let b = string_to_int(Some(substring(s, num_start, num_len)?), 16, IS_TIGHT, None)? as i16;

    if !is_hex_prefix(s, num_start + num_len + 1) {
        return Err(failure("Format_GuidHexPrefix"));
    }
    num_start = num_start + num_len + 3;
    num_len = index_of(s, b',', num_start)? - num_start;
    if num_len <= 0 {
        return Err(failure("Format_GuidComma"));
    }
    let c = string_to_int(Some(substring(s, num_start, num_len)?), 16, IS_TIGHT, None)? as i16;

    if s.len() as i32 <= num_start + num_len + 1 || char_at(s, num_start + num_len + 1)? != u16::from(b'{') {
        return Err(failure("Format_GuidBrace"));
    }

    num_len += 1;
    let mut bytes = [0u8; 8];
    for (i, byte) in bytes.iter_mut().enumerate() {
        if !is_hex_prefix(s, num_start + num_len + 1) {
            return Err(failure("Format_GuidHexPrefix"));
        }
        num_start = num_start + num_len + 3;
        if i < 7 {
            num_len = index_of(s, b',', num_start)? - num_start;
            if num_len <= 0 {
                return Err(failure("Format_GuidComma"));
            }
        } else {
            num_len = index_of(s, b'}', num_start)? - num_start;
            if num_len <= 0 {
                return Err(failure("Format_GuidBraceAfterLastNumber"));
            }
        }
        // (uint)Convert.ToInt32(substring, 16)
        let number = string_to_int(Some(substring(s, num_start, num_len)?), 16, IS_TIGHT, None)? as u32;
        if number > 255 {
            return Err(failure("Overflow_Byte"));
        }
        *byte = number as u8;
    }

    let end = num_start + num_len + 1;
    if end >= s.len() as i32 || char_at(s, end)? != u16::from(b'}') {
        return Err(failure("Format_GuidEndBrace"));
    }
    if end != s.len() as i32 - 1 {
        return Err(failure("Format_ExtraJunkAtEnd"));
    }

    Ok(Guid::from_fields(a, b, c, bytes))
}
