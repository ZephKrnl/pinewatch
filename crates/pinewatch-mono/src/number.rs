// Ported from corert `src/System.Private.CoreLib/shared/System/Number.Parsing.cs` (mono/corert 11136ad5), corefx
// `SByte.cs`, `Byte.cs`, `Int16.cs`, `UInt16.cs`, `Int32.cs`, `UInt32.cs`, `Int64.cs`, `UInt64.cs`, `Single.cs` and
// `Double.cs` (mono/corefx 5b4edb9b): Copyright (c) .NET Foundation and Contributors.
// `NumberFormatInfo.ValidateParseStyleInteger` and `ValidateParseStyleFloatingPoint` from reference source
// `numberformatinfo.cs`: Copyright (c) Microsoft
// Corporation. MIT license; see NOTICES-PORTED-CODE.md.

//! Number parsing: `TryParse` with a style and a format provider for every integer width, `float` and `double`.
//!
//! As in corert, styles within `NumberStyles.Integer` take an optimized path, hexadecimal styles take their own,
//! and everything else goes through the `ParseNumber` state machine. A C# `char*` walk reads `'\0'` past the
//! end; [`at`] returns 0 there, the same sentinel.

use crate::number_to_double::number_to_double;
use crate::{MonoException, NetString, chars, sr};

/// `System.Globalization.NumberStyles`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumberStyles(u32);

impl NumberStyles {
    pub const NONE: Self = Self(0);
    pub const ALLOW_LEADING_WHITE: Self = Self(0x001);
    pub const ALLOW_TRAILING_WHITE: Self = Self(0x002);
    pub const ALLOW_LEADING_SIGN: Self = Self(0x004);
    pub const ALLOW_TRAILING_SIGN: Self = Self(0x008);
    pub const ALLOW_PARENTHESES: Self = Self(0x010);
    pub const ALLOW_DECIMAL_POINT: Self = Self(0x020);
    pub const ALLOW_THOUSANDS: Self = Self(0x040);
    pub const ALLOW_EXPONENT: Self = Self(0x080);
    pub const ALLOW_CURRENCY_SYMBOL: Self = Self(0x100);
    pub const ALLOW_HEX_SPECIFIER: Self = Self(0x200);

    pub const INTEGER: Self = Self(0x007);
    pub const HEX_NUMBER: Self = Self(0x203);
    pub const NUMBER: Self = Self(0x06F);
    pub const FLOAT: Self = Self(0x0A7);
    pub const CURRENCY: Self = Self(0x17F);
    pub const ANY: Self = Self(0x1FF);

    /// Any value, as a C# cast from `int` allows.
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn has(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// The `NumberFormatInfo` fields that number parsing reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormat {
    pub positive_sign: NetString,
    pub negative_sign: NetString,
    pub number_decimal_separator: NetString,
    pub number_group_separator: NetString,
    pub currency_symbol: NetString,
    pub currency_decimal_separator: NetString,
    pub currency_group_separator: NetString,
    pub number_negative_pattern: i32,
    pub positive_infinity_symbol: NetString,
    pub negative_infinity_symbol: NetString,
    pub nan_symbol: NetString,
}

impl NumberFormat {
    /// `NumberFormatInfo.InvariantInfo`.
    pub fn invariant() -> Self {
        Self {
            positive_sign: "+".into(),
            negative_sign: "-".into(),
            number_decimal_separator: ".".into(),
            number_group_separator: ",".into(),
            currency_symbol: "\u{00A4}".into(),
            currency_decimal_separator: ".".into(),
            currency_group_separator: ",".into(),
            number_negative_pattern: 1,
            positive_infinity_symbol: "Infinity".into(),
            negative_infinity_symbol: "-Infinity".into(),
            nan_symbol: "NaN".into(),
        }
    }

    /// `positiveSign == "+" && negativeSign == "-"`, which picks the fast sign check in the integer-style path.
    fn has_invariant_signs(&self) -> bool {
        self.positive_sign == "+" && self.negative_sign == "-"
    }
}

const NUMBER_MAX_DIGITS: i32 = 50;
const INT32_PRECISION: i32 = 10;
const UINT32_PRECISION: i32 = INT32_PRECISION;
const INT64_PRECISION: i32 = 19;
const UINT64_PRECISION: i32 = 20;

/// `Number.NumberBuffer`: digits are kept without the C# NUL terminator.
#[derive(Debug, Default)]
struct NumberBuffer {
    digits: Vec<u16>,
    precision: i32,
    scale: i32,
    sign: bool,
}

fn at(s: &[u16], i: usize) -> u16 {
    s.get(i).copied().unwrap_or(0)
}

/// `Number.IsWhite`.
fn is_white(ch: u16) -> bool {
    ch == 0x20 || ch.wrapping_sub(0x09) <= 0x0D - 0x09
}

/// `Number.IsDigit`.
fn is_digit(ch: u16) -> bool {
    ch.wrapping_sub(u16::from(b'0')) <= 9
}

/// `Number.MatchChars`: the position after `pattern` if it matches at `p`. The pattern is read like a C string,
/// up to its first NUL, and an empty one never matches. A space in the input matches a no-break space in the
/// pattern.
fn match_chars(s: &[u16], p: usize, pattern: &NetString) -> Option<usize> {
    let pattern = pattern.as_units();
    let pattern = &pattern[..pattern.iter().position(|&c| c == 0).unwrap_or(pattern.len())];
    if pattern.is_empty() {
        return None;
    }
    let mut i = p;
    for &expected in pattern {
        let cp = at(s, i);
        if cp != expected && !(expected == 0x00A0 && cp == 0x0020) {
            return None;
        }
        i += 1;
    }
    Some(i)
}

const STATE_SIGN: i32 = 0x0001;
const STATE_PARENS: i32 = 0x0002;
const STATE_DIGITS: i32 = 0x0004;
const STATE_NON_ZERO: i32 = 0x0008;
const STATE_DECIMAL: i32 = 0x0010;
const STATE_CURRENCY: i32 = 0x0020;

/// `Number.ParseNumber` (never called with `AllowHexSpecifier`). Returns whether it succeeded and where parsing
/// stopped.
fn parse_number(
    s: &[u16],
    styles: NumberStyles,
    number: &mut NumberBuffer,
    info: &NumberFormat,
    parse_decimal: bool,
) -> (bool, usize) {
    number.scale = 0;
    number.sign = false;

    let mut curr_symbol: Option<&NetString> = None;
    let dec_sep: &NetString;
    let group_sep: &NetString;
    let parsing_currency = styles.has(NumberStyles::ALLOW_CURRENCY_SYMBOL);
    if parsing_currency {
        curr_symbol = Some(&info.currency_symbol);
        // Match the currency separators first and the number separators on failure.
        dec_sep = &info.currency_decimal_separator;
        group_sep = &info.currency_group_separator;
    } else {
        dec_sep = &info.number_decimal_separator;
        group_sep = &info.number_group_separator;
    }

    let mut state = 0;
    let mut p = 0usize;
    let mut ch = at(s, p);

    loop {
        // Eat whitespace unless we've found a sign which isn't followed by a currency symbol.
        if !is_white(ch)
            || !styles.has(NumberStyles::ALLOW_LEADING_WHITE)
            || ((state & STATE_SIGN) != 0 && (state & STATE_CURRENCY) == 0 && info.number_negative_pattern != 2)
        {
            let sign_allowed = styles.has(NumberStyles::ALLOW_LEADING_SIGN) && (state & STATE_SIGN) == 0;
            let positive = sign_allowed.then(|| match_chars(s, p, &info.positive_sign)).flatten();
            let negative = (sign_allowed && positive.is_none()).then(|| match_chars(s, p, &info.negative_sign)).flatten();
            if let Some(next) = positive.or(negative) {
                if negative.is_some() {
                    number.sign = true;
                }
                state |= STATE_SIGN;
                p = next - 1;
            } else if ch == u16::from(b'(') && styles.has(NumberStyles::ALLOW_PARENTHESES) && (state & STATE_SIGN) == 0 {
                state |= STATE_SIGN | STATE_PARENS;
                number.sign = true;
            } else if let Some(next) = curr_symbol.and_then(|symbol| match_chars(s, p, symbol)) {
                state |= STATE_CURRENCY;
                // There can be only one currency symbol.
                curr_symbol = None;
                p = next - 1;
            } else {
                break;
            }
        }
        p += 1;
        ch = at(s, p);
    }

    let mut dig_count: i32 = 0;
    let mut dig_end: i32 = 0;
    loop {
        if is_digit(ch) {
            state |= STATE_DIGITS;
            if ch != u16::from(b'0') || (state & STATE_NON_ZERO) != 0 {
                if dig_count < NUMBER_MAX_DIGITS {
                    number.digits.push(ch);
                    dig_count += 1;
                    if ch != u16::from(b'0') || parse_decimal {
                        dig_end = dig_count;
                    }
                }
                if (state & STATE_DECIMAL) == 0 {
                    number.scale += 1;
                }
                state |= STATE_NON_ZERO;
            } else if (state & STATE_DECIMAL) != 0 {
                number.scale -= 1;
            }
        } else if let Some(next) = (styles.has(NumberStyles::ALLOW_DECIMAL_POINT) && (state & STATE_DECIMAL) == 0)
            .then(|| {
                match_chars(s, p, dec_sep).or_else(|| {
                    (parsing_currency && (state & STATE_CURRENCY) == 0)
                        .then(|| match_chars(s, p, &info.number_decimal_separator))
                        .flatten()
                })
            })
            .flatten()
        {
            state |= STATE_DECIMAL;
            p = next - 1;
        } else if let Some(next) =
            (styles.has(NumberStyles::ALLOW_THOUSANDS) && (state & STATE_DIGITS) != 0 && (state & STATE_DECIMAL) == 0)
                .then(|| {
                    match_chars(s, p, group_sep).or_else(|| {
                        (parsing_currency && (state & STATE_CURRENCY) == 0)
                            .then(|| match_chars(s, p, &info.number_group_separator))
                            .flatten()
                    })
                })
                .flatten()
        {
            p = next - 1;
        } else {
            break;
        }
        p += 1;
        ch = at(s, p);
    }

    number.precision = dig_end;
    number.digits.truncate(dig_end as usize);

    if (state & STATE_DIGITS) != 0 {
        if (ch == u16::from(b'E') || ch == u16::from(b'e')) && styles.has(NumberStyles::ALLOW_EXPONENT) {
            let temp = p;
            let mut neg_exp = false;
            p += 1;
            ch = at(s, p);
            if let Some(next) = match_chars(s, p, &info.positive_sign) {
                p = next;
                ch = at(s, p);
            } else if let Some(next) = match_chars(s, p, &info.negative_sign) {
                p = next;
                ch = at(s, p);
                neg_exp = true;
            }
            if is_digit(ch) {
                let mut exp: i32 = 0;
                loop {
                    exp = exp * 10 + i32::from(ch - u16::from(b'0'));
                    p += 1;
                    ch = at(s, p);
                    if exp > 1000 {
                        exp = 9999;
                        while is_digit(ch) {
                            p += 1;
                            ch = at(s, p);
                        }
                    }
                    if !is_digit(ch) {
                        break;
                    }
                }
                if neg_exp {
                    exp = -exp;
                }
                number.scale += exp;
            } else {
                p = temp;
                ch = at(s, p);
            }
        }

        loop {
            if !is_white(ch) || !styles.has(NumberStyles::ALLOW_TRAILING_WHITE) {
                let sign_allowed = styles.has(NumberStyles::ALLOW_TRAILING_SIGN) && (state & STATE_SIGN) == 0;
                let positive = sign_allowed.then(|| match_chars(s, p, &info.positive_sign)).flatten();
                let negative =
                    (sign_allowed && positive.is_none()).then(|| match_chars(s, p, &info.negative_sign)).flatten();
                if let Some(next) = positive.or(negative) {
                    if negative.is_some() {
                        number.sign = true;
                    }
                    state |= STATE_SIGN;
                    p = next - 1;
                } else if ch == u16::from(b')') && (state & STATE_PARENS) != 0 {
                    state &= !STATE_PARENS;
                } else if let Some(next) = curr_symbol.and_then(|symbol| match_chars(s, p, symbol)) {
                    curr_symbol = None;
                    p = next - 1;
                } else {
                    break;
                }
            }
            p += 1;
            ch = at(s, p);
        }

        if (state & STATE_PARENS) == 0 {
            if (state & STATE_NON_ZERO) == 0 {
                if !parse_decimal {
                    number.scale = 0;
                }
                if (state & STATE_DECIMAL) == 0 {
                    number.sign = false;
                }
            }
            return (true, p);
        }
    }
    (false, p)
}

/// `Number.TrailingZeros`: only NUL characters may follow the number.
fn trailing_zeros(value: &[u16], index: usize) -> bool {
    value[index..].iter().all(|&c| c == 0)
}

/// `Number.TryStringToNumber`.
fn try_string_to_number(
    value: &[u16],
    styles: NumberStyles,
    info: &NumberFormat,
    parse_decimal: bool,
) -> Option<NumberBuffer> {
    let mut number = NumberBuffer::default();
    let (ok, p) = parse_number(value, styles, &mut number, info, parse_decimal);
    if !ok || (p < value.len() && !trailing_zeros(value, p)) {
        return None;
    }
    Some(number)
}

fn digit_value(d: u16) -> u16 {
    d - u16::from(b'0')
}

/// `Number.NumberToInt32`, with C#'s unchecked (wrapping) arithmetic.
fn number_to_i32(number: &NumberBuffer) -> Option<i32> {
    let mut i = number.scale;
    if i > INT32_PRECISION || i < number.precision {
        return None;
    }
    let mut digits = number.digits.iter();
    let mut n: i32 = 0;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        if (n as u32) > (0x7FFF_FFFF / 10) {
            return None;
        }
        n = n.wrapping_mul(10);
        if let Some(&d) = digits.next() {
            n = n.wrapping_add(i32::from(digit_value(d)));
        }
    }
    if number.sign {
        n = n.wrapping_neg();
        if n > 0 {
            return None;
        }
    } else if n < 0 {
        return None;
    }
    Some(n)
}

/// `Number.NumberToUInt32`.
fn number_to_u32(number: &NumberBuffer) -> Option<u32> {
    let mut i = number.scale;
    if i > UINT32_PRECISION || i < number.precision || number.sign {
        return None;
    }
    let mut digits = number.digits.iter();
    let mut n: u32 = 0;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        if n > (0xFFFF_FFFF / 10) {
            return None;
        }
        n = n.wrapping_mul(10);
        if let Some(&d) = digits.next() {
            let new_n = n.wrapping_add(u32::from(digit_value(d)));
            if new_n < n {
                return None;
            }
            n = new_n;
        }
    }
    Some(n)
}

/// `Number.NumberToInt64`.
fn number_to_i64(number: &NumberBuffer) -> Option<i64> {
    let mut i = number.scale;
    if i > INT64_PRECISION || i < number.precision {
        return None;
    }
    let mut digits = number.digits.iter();
    let mut n: i64 = 0;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        if (n as u64) > (0x7FFF_FFFF_FFFF_FFFF / 10) {
            return None;
        }
        n = n.wrapping_mul(10);
        if let Some(&d) = digits.next() {
            n = n.wrapping_add(i64::from(digit_value(d)));
        }
    }
    if number.sign {
        n = n.wrapping_neg();
        if n > 0 {
            return None;
        }
    } else if n < 0 {
        return None;
    }
    Some(n)
}

/// `Number.NumberToUInt64`.
fn number_to_u64(number: &NumberBuffer) -> Option<u64> {
    let mut i = number.scale;
    if i > UINT64_PRECISION || i < number.precision || number.sign {
        return None;
    }
    let mut digits = number.digits.iter();
    let mut n: u64 = 0;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        if n > (u64::MAX / 10) {
            return None;
        }
        n = n.wrapping_mul(10);
        if let Some(&d) = digits.next() {
            let new_n = n.wrapping_add(u64::from(digit_value(d)));
            if new_n < n {
                return None;
            }
            n = new_n;
        }
    }
    Some(n)
}

/// Where the integer-style parsers end up: `DoneAtEndButPotentialOverflow` or `FalseExit`. Overflow is a failure
/// for `TryParse` either way; `Parse` would turn it into an `OverflowException` instead of a `FormatException`.
enum IntegerParse {
    Done,
    False,
}

/// The shared front of the integer-style parsers: leading whitespace and the sign. Returns where the digits
/// start (in the possibly re-sliced `value`), whether a negative sign was read, or `None` for `FalseExit`.
fn integer_style_prefix<'v>(
    mut value: &'v [u16],
    styles: NumberStyles,
    info: &NumberFormat,
) -> Option<(&'v [u16], usize, bool)> {
    if value.is_empty() {
        return None;
    }
    let mut negative = false;
    let mut index = 0usize;
    let mut num = value[0];

    if styles.has(NumberStyles::ALLOW_LEADING_WHITE) && is_white(num) {
        loop {
            index += 1;
            if index >= value.len() {
                return None;
            }
            num = value[index];
            if !is_white(num) {
                break;
            }
        }
    }

    if styles.has(NumberStyles::ALLOW_LEADING_SIGN) {
        if info.has_invariant_signs() {
            if num == u16::from(b'-') || num == u16::from(b'+') {
                negative = num == u16::from(b'-');
                index += 1;
                if index >= value.len() {
                    return None;
                }
            }
        } else {
            value = &value[index..];
            index = 0;
            let positive = info.positive_sign.as_units();
            let negative_sign = info.negative_sign.as_units();
            // ReadOnlySpan<char>.StartsWith: an ordinal comparison of the whole sign string.
            if !positive.is_empty() && value.starts_with(positive) {
                index += positive.len();
                if index >= value.len() {
                    return None;
                }
            } else if !negative_sign.is_empty() && value.starts_with(negative_sign) {
                negative = true;
                index += negative_sign.len();
                if index >= value.len() {
                    return None;
                }
            }
        }
    }
    Some((value, index, negative))
}

/// `HasTrailingChars`: trailing whitespace if allowed, then only NULs.
fn integer_style_trailing(value: &[u16], mut index: usize, styles: NumberStyles) -> IntegerParse {
    if is_white(value[index]) {
        if !styles.has(NumberStyles::ALLOW_TRAILING_WHITE) {
            return IntegerParse::False;
        }
        index += 1;
        while index < value.len() && is_white(value[index]) {
            index += 1;
        }
        if index >= value.len() {
            return IntegerParse::Done;
        }
    }
    if trailing_zeros(value, index) { IntegerParse::Done } else { IntegerParse::False }
}

/// Decimal digits for the integer-style parsers: skips leading zeros, then reads the `safe_digits` significant digits
/// that can't overflow (9 for 32 bits, 18 for `Int64`, 19 for `UInt64`). Returns their value (C#'s wrapping
/// arithmetic, so the bits match for every width), the next digit if there is one (for the caller's overflow check),
/// and whether more digits follow. `None` is `FalseExit`: no digit at all.
fn integer_style_digits(
    value: &[u16],
    mut index: usize,
    styles: NumberStyles,
    safe_digits: usize,
) -> Option<(u64, Option<u16>, bool, IntegerParse)> {
    let mut num = value[index];
    if !is_digit(num) {
        return None;
    }
    // Skip past leading zeros.
    if num == u16::from(b'0') {
        loop {
            index += 1;
            if index >= value.len() {
                return Some((0, None, false, IntegerParse::Done));
            }
            num = value[index];
            if num != u16::from(b'0') {
                break;
            }
        }
        if !is_digit(num) {
            return Some((0, None, false, integer_style_trailing(value, index, styles)));
        }
    }

    // The first `safe_digits` digits can't overflow.
    let mut answer = u64::from(num - u16::from(b'0'));
    index += 1;
    for _ in 1..safe_digits {
        if index >= value.len() {
            return Some((answer, None, false, IntegerParse::Done));
        }
        num = value[index];
        if !is_digit(num) {
            return Some((answer, None, false, integer_style_trailing(value, index, styles)));
        }
        index += 1;
        answer = answer.wrapping_mul(10).wrapping_add(u64::from(num - u16::from(b'0')));
    }

    // The next digit may overflow; the caller checks.
    if index >= value.len() {
        return Some((answer, None, false, IntegerParse::Done));
    }
    num = value[index];
    if !is_digit(num) {
        return Some((answer, None, false, integer_style_trailing(value, index, styles)));
    }
    index += 1;
    let last = num;
    if index >= value.len() {
        return Some((answer, Some(last), false, IntegerParse::Done));
    }

    // Any further digit overflows, but format errors take precedence for compatibility.
    num = value[index];
    let mut more = false;
    while is_digit(num) {
        more = true;
        index += 1;
        if index >= value.len() {
            return Some((answer, Some(last), more, IntegerParse::Done));
        }
        num = value[index];
    }
    Some((answer, Some(last), more, integer_style_trailing(value, index, styles)))
}

/// `Number.TryParseInt32IntegerStyle`.
fn try_parse_int32_integer_style(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<i32> {
    let (value, index, negative) = integer_style_prefix(value, styles, info)?;
    let sign: i32 = if negative { -1 } else { 1 };
    let (safe, last, more, end) = integer_style_digits(value, index, styles, 9)?;

    let mut overflow = more;
    let mut answer = safe as i32;
    if let Some(last) = last {
        if answer > i32::MAX / 10 {
            overflow = true;
        }
        answer = answer.wrapping_mul(10).wrapping_add(i32::from(last - u16::from(b'0')));
        // (uint)answer > (uint)int.MaxValue + (-1 * sign + 1) / 2
        if answer as u32 > (i32::MAX as u32).wrapping_add(((-sign + 1) / 2) as u32) {
            overflow = true;
        }
    }
    match end {
        IntegerParse::Done if !overflow => Some(answer.wrapping_mul(sign)),
        _ => None,
    }
}

/// `Number.TryParseUInt32IntegerStyle`.
fn try_parse_uint32_integer_style(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<u32> {
    let (value, index, has_negative_sign) = integer_style_prefix(value, styles, info)?;
    let (safe, last, more, end) = integer_style_digits(value, index, styles, 9)?;

    let mut overflow = more;
    let mut answer = safe as u32;
    if let Some(last) = last {
        if answer > u32::MAX / 10 || (answer == u32::MAX / 10 && last > u16::from(b'5')) {
            overflow = true;
        }
        answer = answer.wrapping_mul(10).wrapping_add(u32::from(last - u16::from(b'0')));
    }
    match end {
        // A negative sign is only allowed on zero.
        IntegerParse::Done if !overflow && !(has_negative_sign && answer != 0) => Some(answer),
        _ => None,
    }
}

/// `Number.TryParseInt64IntegerStyle`.
fn try_parse_int64_integer_style(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<i64> {
    let (value, index, negative) = integer_style_prefix(value, styles, info)?;
    let sign: i64 = if negative { -1 } else { 1 };
    let (safe, last, more, end) = integer_style_digits(value, index, styles, 18)?;

    let mut overflow = more;
    let mut answer = safe as i64;
    if let Some(last) = last {
        if answer > i64::MAX / 10 {
            overflow = true;
        }
        answer = answer.wrapping_mul(10).wrapping_add(i64::from(last - u16::from(b'0')));
        // (ulong)answer > (ulong)long.MaxValue + (ulong)((-1 * sign + 1) / 2)
        if answer as u64 > (i64::MAX as u64).wrapping_add(((-sign + 1) / 2) as u64) {
            overflow = true;
        }
    }
    match end {
        IntegerParse::Done if !overflow => Some(answer.wrapping_mul(sign)),
        _ => None,
    }
}

/// `Number.TryParseUInt64IntegerStyle`. C# accumulates in a `long` that wraps; `u64` wrapping arithmetic gives the
/// same bits.
fn try_parse_uint64_integer_style(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<u64> {
    let (value, index, has_negative_sign) = integer_style_prefix(value, styles, info)?;
    let (safe, last, more, end) = integer_style_digits(value, index, styles, 19)?;

    let mut overflow = more;
    let mut answer = safe;
    if let Some(last) = last {
        if answer > u64::MAX / 10 || (answer == u64::MAX / 10 && last > u16::from(b'5')) {
            overflow = true;
        }
        answer = answer.wrapping_mul(10).wrapping_add(u64::from(last - u16::from(b'0')));
    }
    match end {
        IntegerParse::Done if !overflow && !(has_negative_sign && answer != 0) => Some(answer),
        _ => None,
    }
}

/// `s_charToHexLookup[num]` for `num` below 256: the value of an ASCII hex digit.
fn hex_value(num: u16) -> Option<u64> {
    match num {
        0x30..=0x39 => Some(u64::from(num - 0x30)),
        0x41..=0x46 => Some(u64::from(num - 0x41) + 10),
        0x61..=0x66 => Some(u64::from(num - 0x61) + 10),
        _ => None,
    }
}

/// `Number.TryParseUInt32HexNumberStyle` (`max_digits` 8) and `TryParseUInt64HexNumberStyle` (`max_digits` 16):
/// that many significant hex digits fit, one more is an overflow.
fn hex_number_style(value: &[u16], styles: NumberStyles, max_digits: usize) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let mut index = 0usize;
    let mut num = value[0];

    if styles.has(NumberStyles::ALLOW_LEADING_WHITE) && is_white(num) {
        loop {
            index += 1;
            if index >= value.len() {
                return None;
            }
            num = value[index];
            if !is_white(num) {
                break;
            }
        }
    }

    // Not a hex digit: FalseExit.
    hex_value(num)?;
    let mut overflow = false;
    let mut answer: u64 = 0;
    let end = 'digits: {
        // Skip past leading zeros.
        if num == u16::from(b'0') {
            loop {
                index += 1;
                if index >= value.len() {
                    return Some(0);
                }
                num = value[index];
                if num != u16::from(b'0') {
                    break;
                }
            }
        }
        // Without leading zeros `num` is the hex digit checked above; after them it may be anything.
        let Some(first_digit) = hex_value(num) else {
            break 'digits integer_style_trailing(value, index, styles);
        };

        // Up to `max_digits` digits can't overflow.
        answer = first_digit;
        index += 1;
        for _ in 1..max_digits {
            if index >= value.len() {
                return Some(answer);
            }
            num = value[index];
            let Some(digit) = hex_value(num) else {
                break 'digits integer_style_trailing(value, index, styles);
            };
            index += 1;
            answer = answer.wrapping_mul(16).wrapping_add(digit);
        }

        // Another digit is an overflow.
        if index >= value.len() {
            return Some(answer);
        }
        num = value[index];
        if hex_value(num).is_none() {
            break 'digits integer_style_trailing(value, index, styles);
        }
        index += 1;
        overflow = true;
        if index >= value.len() {
            break 'digits IntegerParse::Done;
        }

        // Format errors take precedence: read through any remaining digits.
        num = value[index];
        while hex_value(num).is_some() {
            index += 1;
            if index >= value.len() {
                break 'digits IntegerParse::Done;
            }
            num = value[index];
        }
        integer_style_trailing(value, index, styles)
    };

    match end {
        IntegerParse::Done if !overflow => Some(answer),
        _ => None,
    }
}

/// `Number.TryParseInt32`.
fn number_try_parse_int32(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<i32> {
    if styles.bits() & !NumberStyles::INTEGER.bits() == 0 {
        return try_parse_int32_integer_style(value, styles, info);
    }
    if styles.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        return hex_number_style(value, styles, 8).map(|v| v as u32 as i32);
    }
    number_to_i32(&try_string_to_number(value, styles, info, false)?)
}

/// `Number.TryParseUInt32`.
fn number_try_parse_uint32(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<u32> {
    if styles.bits() & !NumberStyles::INTEGER.bits() == 0 {
        return try_parse_uint32_integer_style(value, styles, info);
    }
    if styles.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        return hex_number_style(value, styles, 8).map(|v| v as u32);
    }
    number_to_u32(&try_string_to_number(value, styles, info, false)?)
}

/// `Number.TryParseInt64`.
fn number_try_parse_int64(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<i64> {
    if styles.bits() & !NumberStyles::INTEGER.bits() == 0 {
        return try_parse_int64_integer_style(value, styles, info);
    }
    if styles.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        return hex_number_style(value, styles, 16).map(|v| v as i64);
    }
    number_to_i64(&try_string_to_number(value, styles, info, false)?)
}

/// `Number.TryParseUInt64`.
fn number_try_parse_uint64(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<u64> {
    if styles.bits() & !NumberStyles::INTEGER.bits() == 0 {
        return try_parse_uint64_integer_style(value, styles, info);
    }
    if styles.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        return hex_number_style(value, styles, 16);
    }
    number_to_u64(&try_string_to_number(value, styles, info, false)?)
}

/// `NumberFormatInfo.ValidateParseStyleInteger` (reference source).
fn validate_parse_style_integer(style: NumberStyles) -> Result<(), MonoException> {
    // InvalidNumberStyles: every bit above AllowHexSpecifier.
    if style.bits() & !0x3FF != 0 {
        return Err(MonoException::argument(sr::ARGUMENT_INVALID_NUMBER_STYLES, Some("style")));
    }
    if style.has(NumberStyles::ALLOW_HEX_SPECIFIER) && style.bits() & !NumberStyles::HEX_NUMBER.bits() != 0 {
        return Err(MonoException::argument(sr::ARG_INVALID_HEX_STYLE, None));
    }
    Ok(())
}

/// `Int32.TryParse(string s, NumberStyles style, IFormatProvider provider, out int result)`: an invalid style
/// throws (even for a null `s`); otherwise `Ok(None)` is `false`. A missing string (C# `null`) fails.
pub fn try_parse_i32(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<i32>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_int32(s.as_units(), style, info)))
}

/// `UInt32.TryParse(string s, NumberStyles style, IFormatProvider provider, out uint result)`.
pub fn try_parse_u32(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<u32>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_uint32(s.as_units(), style, info)))
}

/// `SByte.TryParse(string, NumberStyles, IFormatProvider, out sbyte)`: `Number.TryParseInt32`, then a range check. A
/// hexadecimal value must be 0 to 255 and keeps its bits, so `"80"` is -128.
pub fn try_parse_i8(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<i8>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_int32(s.as_units(), style, info)).and_then(|i| {
        if style.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
            (0..=i32::from(u8::MAX)).contains(&i).then_some(i as i8)
        } else {
            i8::try_from(i).ok()
        }
    }))
}

/// `Byte.TryParse`: `Number.TryParseInt32`, then 0 to 255.
pub fn try_parse_u8(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<u8>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_int32(s.as_units(), style, info)).and_then(|i| u8::try_from(i).ok()))
}

/// `Int16.TryParse`: like `SByte`, with a 0 to 65535 hexadecimal range.
pub fn try_parse_i16(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<i16>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_int32(s.as_units(), style, info)).and_then(|i| {
        if style.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
            (0..=i32::from(u16::MAX)).contains(&i).then_some(i as i16)
        } else {
            i16::try_from(i).ok()
        }
    }))
}

/// `UInt16.TryParse`: `Number.TryParseUInt32` (not `Int32`), then 0 to 65535.
pub fn try_parse_u16(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<u16>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_uint32(s.as_units(), style, info)).and_then(|i| u16::try_from(i).ok()))
}

/// `Int64.TryParse(string s, NumberStyles style, IFormatProvider provider, out long result)`.
pub fn try_parse_i64(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<i64>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_int64(s.as_units(), style, info)))
}

/// `UInt64.TryParse(string s, NumberStyles style, IFormatProvider provider, out ulong result)`.
pub fn try_parse_u64(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<u64>, MonoException> {
    validate_parse_style_integer(style)?;
    Ok(s.and_then(|s| number_try_parse_uint64(s.as_units(), style, info)))
}

/// `Number.NumberBufferToDouble`: fails for an infinite result and turns -0.0 into 0.0.
fn number_buffer_to_double(number: &NumberBuffer) -> Option<f64> {
    let d = number_to_double(&number.digits, number.scale, number.sign);
    if !d.is_finite() {
        return None;
    }
    Some(if d == 0.0 { 0.0 } else { d })
}

/// `Number.TryParseDouble`.
fn number_try_parse_double(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<f64> {
    let number = try_string_to_number(value, styles, info, false)?;
    number_buffer_to_double(&number)
}

/// `Number.TryParseSingle`: the `(float)` cast of the double, which fails when the cast is infinite.
fn number_try_parse_single(value: &[u16], styles: NumberStyles, info: &NumberFormat) -> Option<f32> {
    let cast = number_try_parse_double(value, styles, info)? as f32;
    if cast.is_infinite() {
        return None;
    }
    Some(cast)
}

/// `NumberFormatInfo.ValidateParseStyleFloatingPoint` (reference source).
fn validate_parse_style_floating_point(style: NumberStyles) -> Result<(), MonoException> {
    if style.bits() & !0x3FF != 0 {
        return Err(MonoException::argument(sr::ARGUMENT_INVALID_NUMBER_STYLES, Some("style")));
    }
    if style.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        return Err(MonoException::argument(sr::ARG_HEX_STYLE_NOT_SUPPORTED, None));
    }
    Ok(())
}

/// `Double.NaN` and `Single.NaN` as the compiled constants hold them: `0.0 / 0.0` folded at compile time, a negative
/// quiet NaN (`ldc.r8` and `ldc.r4` in the shipped `Double.TryParse` and `Single.TryParse`).
const DOUBLE_NAN_BITS: u64 = 0xFFF8_0000_0000_0000;
const SINGLE_NAN_BITS: u32 = 0xFFC0_0000;

/// `double.NaN`, with the bits the compiled constant holds (Rust's `f64::NAN` has the sign bit clear).
pub const DOUBLE_NAN: f64 = f64::from_bits(DOUBLE_NAN_BITS);

/// The fallback of `Double.TryParse` and `Single.TryParse` when number parsing fails: the input, trimmed with
/// `char.IsWhiteSpace`, equal (ordinal) to the positive infinity, negative infinity or NaN symbol, in that order.
fn special_symbol(value: &[u16], info: &NumberFormat) -> Option<SpecialValue> {
    let start = value.iter().position(|&c| !chars::is_white_space(c)).unwrap_or(value.len());
    let end = value.iter().rposition(|&c| !chars::is_white_space(c)).map_or(start, |i| i + 1);
    let trimmed = &value[start..end.max(start)];
    if trimmed == info.positive_infinity_symbol.as_units() {
        Some(SpecialValue::PositiveInfinity)
    } else if trimmed == info.negative_infinity_symbol.as_units() {
        Some(SpecialValue::NegativeInfinity)
    } else if trimmed == info.nan_symbol.as_units() {
        Some(SpecialValue::NaN)
    } else {
        None
    }
}

enum SpecialValue {
    PositiveInfinity,
    NegativeInfinity,
    NaN,
}

/// `Double.TryParse(string s, NumberStyles style, IFormatProvider provider, out double result)`.
pub fn try_parse_f64(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<f64>, MonoException> {
    validate_parse_style_floating_point(style)?;
    let Some(s) = s else { return Ok(None) };
    let value = s.as_units();
    Ok(number_try_parse_double(value, style, info).or_else(|| {
        special_symbol(value, info).map(|special| match special {
            SpecialValue::PositiveInfinity => f64::INFINITY,
            SpecialValue::NegativeInfinity => f64::NEG_INFINITY,
            SpecialValue::NaN => f64::from_bits(DOUBLE_NAN_BITS),
        })
    }))
}

/// `Single.TryParse(string s, NumberStyles style, IFormatProvider provider, out float result)`.
pub fn try_parse_f32(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Result<Option<f32>, MonoException> {
    validate_parse_style_floating_point(style)?;
    let Some(s) = s else { return Ok(None) };
    let value = s.as_units();
    Ok(number_try_parse_single(value, style, info).or_else(|| {
        special_symbol(value, info).map(|special| match special {
            SpecialValue::PositiveInfinity => f32::INFINITY,
            SpecialValue::NegativeInfinity => f32::NEG_INFINITY,
            SpecialValue::NaN => f32::from_bits(SINGLE_NAN_BITS),
        })
    }))
}
