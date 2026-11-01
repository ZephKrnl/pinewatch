// Ported from corefx `src/Common/src/CoreLib/System/ParseNumbers.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! `System.ParseNumbers`: `StringToInt` and `StringToLong`, which `Guid` (and later `Convert.ToInt32(string,
//! fromBase)`) use. Digits are ASCII only, and parsing stops at the first character that isn't a digit in the
//! radix.

use crate::{MonoException, chars, sr};

pub(crate) const TREAT_AS_UNSIGNED: i32 = 0x0200;
pub(crate) const TREAT_AS_I1: i32 = 0x0400;
pub(crate) const TREAT_AS_I2: i32 = 0x0800;
pub(crate) const IS_TIGHT: i32 = 0x1000;
pub(crate) const NO_SPACE: i32 = 0x2000;

/// The radix check at the top of both methods: -1 means "decimal unless a 0x prefix says hexadecimal".
fn check_radix(radix: i32) -> Result<i32, MonoException> {
    let r = if radix == -1 { 10 } else { radix };
    if r != 2 && r != 10 && r != 8 && r != 16 {
        return Err(MonoException::argument(sr::ARG_INVALID_BASE, Some("radix")));
    }
    Ok(r)
}

/// The checks both methods make before reading digits. Returns the sign and the possibly updated radix, and
/// leaves `i` at the first digit.
fn parse_prefix(s: &[u16], radix: i32, flags: i32, i: &mut i32) -> Result<(i32, i32), MonoException> {
    let mut r = check_radix(radix)?;
    let length = s.len() as i32;
    if *i < 0 || *i >= length {
        // The one-argument constructor takes a parameter name, so this message ends up after "Parameter name:".
        return Err(MonoException::argument_out_of_range(sr::ARGUMENT_OUT_OF_RANGE_INDEX));
    }

    if flags & IS_TIGHT == 0 && flags & NO_SPACE == 0 {
        eat_white_space(s, i);
        if *i == length {
            return Err(MonoException::format(sr::FORMAT_EMPTY_INPUT_STRING));
        }
    }

    let mut sign = 1;
    if s[*i as usize] == u16::from(b'-') {
        if r != 10 {
            return Err(MonoException::argument(sr::ARG_CANNOT_HAVE_NEGATIVE_VALUE, None));
        }
        if flags & TREAT_AS_UNSIGNED != 0 {
            return Err(MonoException::overflow_message(sr::OVERFLOW_NEGATIVE_UNSIGNED));
        }
        sign = -1;
        *i += 1;
    } else if s[*i as usize] == u16::from(b'+') {
        *i += 1;
    }

    // Consume the 0x if we're in an unknown base or in base 16 (the radix argument, not `r`).
    if (radix == -1 || radix == 16) && *i + 1 < length && s[*i as usize] == u16::from(b'0') {
        let x = s[(*i + 1) as usize];
        if x == u16::from(b'x') || x == u16::from(b'X') {
            r = 16;
            *i += 2;
        }
    }
    Ok((sign, r))
}

/// `ParseNumbers.StringToInt(s, radix, flags, ref currPos)`.
pub(crate) fn string_to_int(s: &[u16], radix: i32, flags: i32, curr_pos: &mut i32) -> Result<i32, MonoException> {
    let mut i = *curr_pos;
    let (sign, r) = parse_prefix(s, radix, flags, &mut i)?;

    let grab_numbers_start = i;
    let result = grab_ints(r, s, &mut i, flags & TREAT_AS_UNSIGNED != 0)?;
    if i == grab_numbers_start {
        return Err(MonoException::format(sr::FORMAT_NO_PARSIBLE_DIGITS));
    }
    if flags & IS_TIGHT != 0 && i < s.len() as i32 {
        return Err(MonoException::format(sr::FORMAT_EXTRA_JUNK_AT_END));
    }
    *curr_pos = i;

    if flags & TREAT_AS_I1 != 0 {
        if result as u32 > 0xFF {
            return Err(MonoException::overflow_message(sr::OVERFLOW_SBYTE));
        }
    } else if flags & TREAT_AS_I2 != 0 {
        if result as u32 > 0xFFFF {
            return Err(MonoException::overflow_message(sr::OVERFLOW_INT16));
        }
    } else if result as u32 == 0x8000_0000 && sign == 1 && r == 10 && flags & TREAT_AS_UNSIGNED == 0 {
        return Err(MonoException::overflow_message(sr::OVERFLOW_INT32));
    }

    Ok(if r == 10 { result.wrapping_mul(sign) } else { result })
}

/// `ParseNumbers.StringToLong(s, radix, flags, ref currPos)`.
pub(crate) fn string_to_long(s: &[u16], radix: i32, flags: i32, curr_pos: &mut i32) -> Result<i64, MonoException> {
    let mut i = *curr_pos;
    let (sign, r) = parse_prefix(s, radix, flags, &mut i)?;

    let grab_numbers_start = i;
    let result = grab_longs(r, s, &mut i, flags & TREAT_AS_UNSIGNED != 0)?;
    if i == grab_numbers_start {
        return Err(MonoException::format(sr::FORMAT_NO_PARSIBLE_DIGITS));
    }
    if flags & IS_TIGHT != 0 && i < s.len() as i32 {
        return Err(MonoException::format(sr::FORMAT_EXTRA_JUNK_AT_END));
    }
    *curr_pos = i;

    if result as u64 == 0x8000_0000_0000_0000 && sign == 1 && r == 10 && flags & TREAT_AS_UNSIGNED == 0 {
        return Err(MonoException::overflow_message(sr::OVERFLOW_INT64));
    }

    Ok(if r == 10 { result.wrapping_mul(i64::from(sign)) } else { result })
}

/// `EatWhiteSpace`: skips `char.IsWhiteSpace` characters.
fn eat_white_space(s: &[u16], i: &mut i32) {
    while (*i as usize) < s.len() && chars::is_white_space(s[*i as usize]) {
        *i += 1;
    }
}

/// `GrabLongs`. Decimal signed input may reach 0x8000000000000000; the caller rejects it unless negative.
fn grab_longs(radix: i32, s: &[u16], i: &mut i32, is_unsigned: bool) -> Result<i64, MonoException> {
    let mut result: u64 = 0;
    if radix == 10 && !is_unsigned {
        let max_val: u64 = 0x7FFF_FFFF_FFFF_FFFF / 10;
        while let Some(value) = digit_at(s, *i, radix) {
            if result > max_val || (result as i64) < 0 {
                return Err(MonoException::overflow_message(sr::OVERFLOW_INT64));
            }
            result = result.wrapping_mul(radix as u64).wrapping_add(value as u64);
            *i += 1;
        }
        if (result as i64) < 0 && result != 0x8000_0000_0000_0000 {
            return Err(MonoException::overflow_message(sr::OVERFLOW_INT64));
        }
    } else {
        let max_val: u64 = u64::MAX / radix as u64;
        while let Some(value) = digit_at(s, *i, radix) {
            if result > max_val {
                return Err(MonoException::overflow_message(sr::OVERFLOW_UINT64));
            }
            let temp = result.wrapping_mul(radix as u64).wrapping_add(value as u64);
            if temp < result {
                return Err(MonoException::overflow_message(sr::OVERFLOW_UINT64));
            }
            result = temp;
            *i += 1;
        }
    }
    Ok(result as i64)
}

/// `GrabInts`. Decimal signed input may reach 0x80000000; the caller rejects it unless negative.
fn grab_ints(radix: i32, s: &[u16], i: &mut i32, is_unsigned: bool) -> Result<i32, MonoException> {
    let mut result: u32 = 0;
    if radix == 10 && !is_unsigned {
        let max_val: u32 = 0x7FFF_FFFF / 10;
        while let Some(value) = digit_at(s, *i, radix) {
            if result > max_val || (result as i32) < 0 {
                return Err(MonoException::overflow_message(sr::OVERFLOW_INT32));
            }
            result = result.wrapping_mul(radix as u32).wrapping_add(value as u32);
            *i += 1;
        }
        if (result as i32) < 0 && result != 0x8000_0000 {
            return Err(MonoException::overflow_message(sr::OVERFLOW_INT32));
        }
    } else {
        let max_val: u32 = u32::MAX / radix as u32;
        while let Some(value) = digit_at(s, *i, radix) {
            if result > max_val {
                return Err(MonoException::overflow_message(sr::OVERFLOW_UINT32));
            }
            let temp = result.wrapping_mul(radix as u32).wrapping_add(value as u32);
            if temp < result {
                return Err(MonoException::overflow_message(sr::OVERFLOW_UINT32));
            }
            result = temp;
            *i += 1;
        }
    }
    Ok(result as i32)
}

/// `i < s.Length && IsDigit(s[i], radix, out value)`: the digit's value when it is below `radix`.
fn digit_at(s: &[u16], i: i32, radix: i32) -> Option<i32> {
    let c = *s.get(usize::try_from(i).ok()?)?;
    is_digit(c, radix)
}

/// `IsDigit`: ASCII digits and letters only.
fn is_digit(c: u16, radix: i32) -> Option<i32> {
    let tmp = if c.wrapping_sub(u16::from(b'0')) <= 9 {
        i32::from(c - u16::from(b'0'))
    } else if c.wrapping_sub(u16::from(b'A')) <= u16::from(b'Z' - b'A') {
        i32::from(c - u16::from(b'A')) + 10
    } else if c.wrapping_sub(u16::from(b'a')) <= u16::from(b'z' - b'a') {
        i32::from(c - u16::from(b'a')) + 10
    } else {
        return None;
    };
    (tmp < radix).then_some(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExceptionKind, NEW_LINE};

    fn units(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn int(s: &str, radix: i32, flags: i32) -> Result<(i32, i32), (ExceptionKind, String)> {
        let mut pos = 0;
        string_to_int(&units(s), radix, flags, &mut pos).map(|v| (v, pos)).map_err(|e| (e.kind, e.message))
    }

    fn long(s: &str, radix: i32, flags: i32) -> Result<i64, (ExceptionKind, String)> {
        let mut pos = 0;
        string_to_long(&units(s), radix, flags, &mut pos).map_err(|e| (e.kind, e.message))
    }

    fn err(kind: ExceptionKind, message: &str) -> (ExceptionKind, String) {
        (kind, message.to_string())
    }

    #[test]
    fn invalid_radix_is_checked_first() {
        let expected = format!("Invalid Base.{NEW_LINE}Parameter name: radix");
        assert_eq!(int("1", 3, IS_TIGHT), Err((ExceptionKind::Argument, expected.clone())));
        let mut pos = 99;
        assert_eq!(string_to_int(&units("1"), 3, IS_TIGHT, &mut pos).unwrap_err().message, expected);
    }

    #[test]
    fn start_position_outside_the_string_puts_the_index_message_in_the_parameter_name() {
        let expected = format!(
            "Specified argument was out of the range of valid values.{NEW_LINE}Parameter name: \
             Index was out of range. Must be non-negative and less than the size of the collection."
        );
        let mut pos = 1;
        let e = string_to_int(&units("1"), 16, IS_TIGHT, &mut pos).unwrap_err();
        assert_eq!((e.kind, e.message), (ExceptionKind::ArgumentOutOfRange, expected));
        assert_eq!(int("", 16, IS_TIGHT).unwrap_err().0, ExceptionKind::ArgumentOutOfRange);
        let mut negative = -1;
        assert_eq!(
            string_to_long(&units("1"), 16, NO_SPACE, &mut negative).unwrap_err().kind,
            ExceptionKind::ArgumentOutOfRange
        );
    }

    #[test]
    fn whitespace_is_skipped_only_without_tight_or_no_space() {
        assert_eq!(int(" \u{00A0}12", 10, 0), Ok((12, 4)));
        assert_eq!(int("   ", 10, 0), Err(err(ExceptionKind::Format, sr::FORMAT_EMPTY_INPUT_STRING)));
        assert_eq!(int(" 12", 10, NO_SPACE), Err(err(ExceptionKind::Format, sr::FORMAT_NO_PARSIBLE_DIGITS)));
    }

    #[test]
    fn sign_rules() {
        assert_eq!(int("-5", 16, NO_SPACE), Err(err(ExceptionKind::Argument, sr::ARG_CANNOT_HAVE_NEGATIVE_VALUE)));
        assert_eq!(
            int("-5", 10, NO_SPACE | TREAT_AS_UNSIGNED),
            Err(err(ExceptionKind::Overflow, sr::OVERFLOW_NEGATIVE_UNSIGNED))
        );
        assert_eq!(int("-5", 10, NO_SPACE), Ok((-5, 2)));
        assert_eq!(int("+5", 16, NO_SPACE), Ok((5, 2)));
        // Radix -1 starts as decimal, so '-' is allowed; "0x" then switches to 16, which ignores the sign.
        assert_eq!(int("-0x10", -1, NO_SPACE), Ok((16, 5)));
    }

    #[test]
    fn digits_stop_at_the_first_non_digit_in_the_radix() {
        assert_eq!(int("12g", 16, NO_SPACE), Ok((0x12, 2)));
        assert_eq!(int("19", 8, NO_SPACE), Ok((1, 1)));
        assert_eq!(int("12g", 16, IS_TIGHT), Err(err(ExceptionKind::Format, sr::FORMAT_EXTRA_JUNK_AT_END)));
        assert_eq!(int("g", 16, NO_SPACE), Err(err(ExceptionKind::Format, sr::FORMAT_NO_PARSIBLE_DIGITS)));
        // Only ASCII counts: Arabic-Indic one and a fullwidth A are not digits.
        assert_eq!(int("\u{0661}", 10, NO_SPACE), Err(err(ExceptionKind::Format, sr::FORMAT_NO_PARSIBLE_DIGITS)));
        assert_eq!(int("\u{FF21}", 16, NO_SPACE), Err(err(ExceptionKind::Format, sr::FORMAT_NO_PARSIBLE_DIGITS)));
        assert_eq!(int("Zz", 16, NO_SPACE), Err(err(ExceptionKind::Format, sr::FORMAT_NO_PARSIBLE_DIGITS)));
        // "0x" is consumed as a prefix even when no digit follows it.
        assert_eq!(int("0x", 16, NO_SPACE), Err(err(ExceptionKind::Format, sr::FORMAT_NO_PARSIBLE_DIGITS)));
        assert_eq!(int("0", 16, NO_SPACE), Ok((0, 1)));
    }

    #[test]
    fn int_overflow() {
        assert_eq!(int("123456789", 16, IS_TIGHT), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_UINT32)));
        assert_eq!(int("ffffffff", 16, IS_TIGHT), Ok((-1, 8)));
        assert_eq!(int("2147483647", 10, IS_TIGHT), Ok((i32::MAX, 10)));
        assert_eq!(int("2147483648", 10, IS_TIGHT), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_INT32)));
        assert_eq!(int("-2147483648", 10, IS_TIGHT), Ok((i32::MIN, 11)));
        assert_eq!(int("-2147483649", 10, IS_TIGHT), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_INT32)));
        assert_eq!(int("4294967295", 10, IS_TIGHT | TREAT_AS_UNSIGNED), Ok((-1, 10)));
        assert_eq!(
            int("4294967296", 10, IS_TIGHT | TREAT_AS_UNSIGNED),
            Err(err(ExceptionKind::Overflow, sr::OVERFLOW_UINT32))
        );
        assert_eq!(int("100", 16, IS_TIGHT | TREAT_AS_I1), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_SBYTE)));
        assert_eq!(int("10000", 16, IS_TIGHT | TREAT_AS_I2), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_INT16)));
    }

    #[test]
    fn long_overflow() {
        assert_eq!(long("8000000000000000", 16, IS_TIGHT), Ok(i64::MIN));
        assert_eq!(long("10000000000000000", 16, IS_TIGHT), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_UINT64)));
        assert_eq!(long("9223372036854775808", 10, IS_TIGHT), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_INT64)));
        assert_eq!(long("-9223372036854775808", 10, IS_TIGHT), Ok(i64::MIN));
        assert_eq!(long("-9223372036854775809", 10, IS_TIGHT), Err(err(ExceptionKind::Overflow, sr::OVERFLOW_INT64)));
        assert_eq!(long("18446744073709551615", 10, IS_TIGHT | TREAT_AS_UNSIGNED), Ok(-1));
        assert_eq!(
            long("18446744073709551616", 10, IS_TIGHT | TREAT_AS_UNSIGNED),
            Err(err(ExceptionKind::Overflow, sr::OVERFLOW_UINT64))
        );
    }

    #[test]
    fn position_is_only_updated_on_success() {
        let s = units("00000000-0000-0000-0000-00000000000g");
        let mut pos = 24;
        assert_eq!(string_to_long(&s, 16, NO_SPACE, &mut pos), Ok(0));
        assert_eq!(pos, 35);
        let mut pos = 0;
        assert!(string_to_int(&units("12345g"), 16, IS_TIGHT, &mut pos).is_err());
        assert_eq!(pos, 0);
    }
}
