//! Port of Mono's managed `System.ParseNumbers` (`corlib/ReferenceSources/ParseNumbers.cs`):
//! `StringToInt` and `StringToLong`, which `Guid` and `Convert.ToInt32(string, base)` use.

use crate::{ExceptionKind, MonoException, chars};

pub(crate) const TREAT_AS_UNSIGNED: i32 = 0x200;
pub(crate) const TREAT_AS_I1: i32 = 0x400;
pub(crate) const TREAT_AS_I2: i32 = 0x800;
pub(crate) const IS_TIGHT: i32 = 0x1000;
pub(crate) const NO_SPACE: i32 = 0x2000;

const NO_DIGITS: &str = "Could not find any parsable digits.";
const TRAILING_JUNK: &str = "Additional unparsable characters are at the end of the string.";
const MINUS_NOT_BASE_10: &str = "String cannot contain a minus sign if the base is not 10.";

fn index(value: &[u16], i: i32) -> Result<u16, MonoException> {
    usize::try_from(i).ok().and_then(|i| value.get(i).copied()).ok_or_else(MonoException::index_out_of_range)
}

/// The digit value Mono computes: `c - '0'` for numbers, `ToLowerInvariant(c) - 'a' + 10` for letters,
/// or `None` for anything else.
fn digit_value(c: u16) -> Option<i64> {
    if chars::is_number(c) {
        Some(i64::from(c) - i64::from(b'0'))
    } else if chars::is_letter(c) {
        Some(i64::from(chars::to_lower_invariant(c)) - i64::from(b'a') + 10)
    } else {
        None
    }
}

/// `ParseNumbers.StringToInt(value, fromBase, flags, parsePos)`. `None` is C# `null`, which returns 0.
pub(crate) fn string_to_int(
    value: Option<&[u16]>,
    from_base: i32,
    flags: i32,
    parse_pos: Option<&mut i32>,
) -> Result<i32, MonoException> {
    if flags & (IS_TIGHT | NO_SPACE) == 0 {
        return Err(MonoException::new(ExceptionKind::NotImplemented, flags.to_string()));
    }
    let Some(value) = value else { return Ok(0) };

    let len = value.len() as i32;
    if len == 0 {
        // Mono: "Mimic broken .net behaviour"
        return Err(MonoException::argument_out_of_range("Empty string"));
    }

    let mut i = parse_pos.as_deref().copied().unwrap_or(0);
    let mut negative = false;
    let first = index(value, i)?;
    if first == u16::from(b'-') {
        if from_base != 10 {
            return Err(MonoException::new(ExceptionKind::Argument, MINUS_NOT_BASE_10));
        }
        if flags & TREAT_AS_UNSIGNED != 0 {
            return Err(MonoException::new(ExceptionKind::Overflow, "Negative number"));
        }
        negative = true;
        i += 1;
    } else if first == u16::from(b'+') {
        i += 1;
    }

    if from_base == 16
        && i + 1 < len
        && index(value, i)? == u16::from(b'0')
        && matches!(index(value, i + 1)?, 0x78 | 0x58)
    {
        i += 2;
    }

    let max_value: i64 = if flags & TREAT_AS_I1 != 0 {
        i64::from(u8::MAX)
    } else if flags & TREAT_AS_I2 != 0 {
        i64::from(u16::MAX)
    } else {
        i64::from(u32::MAX)
    };

    let mut chars_read = 0;
    let mut result: u32 = 0;
    while i < len {
        let c = index(value, i)?;
        let Some(digit) = digit_value(c) else {
            if i == 0 {
                return Err(MonoException::format(NO_DIGITS));
            }
            if flags & IS_TIGHT != 0 {
                return Err(MonoException::format(TRAILING_JUNK));
            }
            break;
        };
        if digit >= i64::from(from_base) {
            if chars_read > 0 {
                return Err(MonoException::format(TRAILING_JUNK));
            }
            return Err(MonoException::format(NO_DIGITS));
        }
        let res = i64::from(from_base) * i64::from(result) + digit;
        if res > max_value {
            return Err(MonoException::overflow());
        }
        result = res as u32;
        chars_read += 1;
        i += 1;
    }

    if chars_read == 0 {
        return Err(MonoException::format(NO_DIGITS));
    }
    if let Some(pos) = parse_pos {
        *pos = i;
    }
    Ok(if negative { (result as i32).wrapping_neg() } else { result as i32 })
}

const BASE16_MAX_OVERFLOW_FREE_VALUE: u64 = u64::MAX / (16 * 16);
const LONG_MIN_VALUE: u64 = (i64::MAX as u64) + 1;

/// `ParseNumbers.StringToLong(value, fromBase, flags, parsePos)`.
pub(crate) fn string_to_long(
    value: Option<&[u16]>,
    from_base: i32,
    flags: i32,
    parse_pos: Option<&mut i32>,
) -> Result<i64, MonoException> {
    if flags & (IS_TIGHT | NO_SPACE) == 0 {
        return Err(MonoException::new(ExceptionKind::NotImplemented, flags.to_string()));
    }
    let Some(value) = value else { return Ok(0) };

    let from_base_u = from_base as u64;
    let len = value.len() as i32;
    let treat_as_unsigned = flags & TREAT_AS_UNSIGNED != 0;
    if len == 0 {
        return Err(MonoException::argument_out_of_range("Empty string"));
    }

    let mut i = parse_pos.as_deref().copied().unwrap_or(0);
    let mut negative = false;
    let first = index(value, i)?;
    if first == u16::from(b'-') {
        if from_base != 10 {
            return Err(MonoException::new(ExceptionKind::Argument, MINUS_NOT_BASE_10));
        }
        if treat_as_unsigned {
            return Err(MonoException::new(ExceptionKind::Overflow, "Negative number"));
        }
        negative = true;
        i += 1;
    } else if first == u16::from(b'+') {
        i += 1;
    }

    if from_base == 16
        && i + 1 < len
        && index(value, i)? == u16::from(b'0')
        && matches!(index(value, i + 1)?, 0x78 | 0x58)
    {
        i += 2;
    }

    let mut chars_read = 0;
    let mut result: u64 = 0;
    while i < len {
        let c = index(value, i)?;
        let Some(digit) = digit_value(c) else {
            if i == 0 {
                return Err(MonoException::format(NO_DIGITS));
            }
            if flags & IS_TIGHT != 0 {
                return Err(MonoException::format(TRAILING_JUNK));
            }
            break;
        };
        let digit = digit as u64;
        if digit >= from_base_u {
            if chars_read > 0 {
                return Err(MonoException::format(TRAILING_JUNK));
            }
            return Err(MonoException::format(NO_DIGITS));
        }

        if result <= BASE16_MAX_OVERFLOW_FREE_VALUE {
            result = result.wrapping_mul(from_base_u).wrapping_add(digit);
        } else {
            // Split into 32-bit halves to detect overflow, as Mono does.
            let a = (result >> 32).wrapping_mul(from_base_u);
            let b = (result & u64::from(u32::MAX)).wrapping_mul(from_base_u).wrapping_add(digit);
            if (b >> 32).wrapping_add(a) > u64::from(u32::MAX) {
                return Err(MonoException::overflow());
            }
            result = (a << 32).wrapping_add(b);
        }
        chars_read += 1;
        i += 1;
    }

    if chars_read == 0 {
        return Err(MonoException::format(NO_DIGITS));
    }
    if let Some(pos) = parse_pos {
        *pos = i;
    }

    if treat_as_unsigned {
        return Ok(result as i64);
    }
    if !negative {
        if from_base == 10 && result > i64::MAX as u64 {
            return Err(MonoException::overflow());
        }
        return Ok(result as i64);
    }
    if result <= i64::MAX as u64 {
        return Ok(-(result as i64));
    }
    if result > LONG_MIN_VALUE {
        return Err(MonoException::overflow());
    }
    Ok(i64::MIN.wrapping_add((LONG_MIN_VALUE - result) as i64))
}
