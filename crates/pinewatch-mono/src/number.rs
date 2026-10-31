//! Number parsing: port of `System.Number` (`ParseNumber`, `TryStringToNumber`, `NumberToInt32`,
//! `NumberToUInt32`, `HexNumberToUInt32`) from Mono's reference source.
//!
//! C# walks a NUL-terminated `char*`; here [`at`] returns 0 past the end, which is the same sentinel.

use crate::NetString;

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

/// The `NumberFormatInfo` fields that parsing reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormat {
    pub positive_sign: NetString,
    pub negative_sign: NetString,
    pub number_decimal_separator: NetString,
    pub number_group_separator: NetString,
    pub currency_symbol: NetString,
    pub currency_decimal_separator: NetString,
    pub currency_group_separator: NetString,
    /// Only ja-JP and ko-KR have one (a backslash).
    pub ansi_currency_symbol: Option<NetString>,
    pub number_negative_pattern: i32,
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
            ansi_currency_symbol: None,
            number_negative_pattern: 1,
        }
    }
}

const NUMBER_MAX_DIGITS: i32 = 50;
const INT32_PRECISION: i32 = 10;
const UINT32_PRECISION: i32 = INT32_PRECISION;

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
    ch == 0x20 || (0x09..=0x0D).contains(&ch)
}

fn is_digit(ch: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&ch)
}

/// `Number.MatchChars`: the position after `pattern` if it matches at `p`. A space in the input matches a
/// no-break space in the pattern. An empty pattern never matches.
fn match_chars(s: &[u16], p: usize, pattern: &[u16]) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    let mut i = p;
    for &expected in pattern {
        let ch = at(s, i);
        if ch != expected && !(expected == 0x00A0 && ch == 0x0020) {
            return None;
        }
        i += 1;
    }
    Some(i)
}

fn match_opt(s: &[u16], p: usize, pattern: Option<&NetString>) -> Option<usize> {
    pattern.and_then(|pattern| match_chars(s, p, pattern.as_units()))
}

const STATE_SIGN: i32 = 0x0001;
const STATE_PARENS: i32 = 0x0002;
const STATE_DIGITS: i32 = 0x0004;
const STATE_NON_ZERO: i32 = 0x0008;
const STATE_DECIMAL: i32 = 0x0010;
const STATE_CURRENCY: i32 = 0x0020;

/// `Number.ParseNumber`. Returns whether it succeeded and where parsing stopped.
fn parse_number(
    s: &[u16],
    options: NumberStyles,
    number: &mut NumberBuffer,
    numfmt: &NumberFormat,
    parse_decimal: bool,
) -> (bool, usize) {
    number.scale = 0;
    number.sign = false;

    let mut curr_symbol: Option<&NetString> = None;
    let mut ansicurr_symbol: Option<&NetString> = None;
    let mut altdec_sep: Option<&NetString> = None;
    let mut altgroup_sep: Option<&NetString> = None;
    let dec_sep: &NetString;
    let group_sep: &NetString;
    let parsing_currency = options.has(NumberStyles::ALLOW_CURRENCY_SYMBOL);
    if parsing_currency {
        curr_symbol = Some(&numfmt.currency_symbol);
        ansicurr_symbol = numfmt.ansi_currency_symbol.as_ref();
        altdec_sep = Some(&numfmt.number_decimal_separator);
        altgroup_sep = Some(&numfmt.number_group_separator);
        dec_sep = &numfmt.currency_decimal_separator;
        group_sep = &numfmt.currency_group_separator;
    } else {
        dec_sep = &numfmt.number_decimal_separator;
        group_sep = &numfmt.number_group_separator;
    }

    let mut state = 0;
    let mut p = 0usize;
    let mut ch = at(s, p);

    loop {
        // Whitespace is eaten unless a sign was found that isn't followed by a currency symbol.
        let white_ok = is_white(ch)
            && options.has(NumberStyles::ALLOW_LEADING_WHITE)
            && ((state & STATE_SIGN) == 0
                || ((state & STATE_CURRENCY) != 0 || numfmt.number_negative_pattern == 2));
        if !white_ok {
            let signflag = options.has(NumberStyles::ALLOW_LEADING_SIGN) && (state & STATE_SIGN) == 0;
            if let Some(next) = match_chars(s, p, numfmt.positive_sign.as_units()).filter(|_| signflag) {
                state |= STATE_SIGN;
                p = next - 1;
            } else if let Some(next) = match_chars(s, p, numfmt.negative_sign.as_units()).filter(|_| signflag) {
                state |= STATE_SIGN;
                number.sign = true;
                p = next - 1;
            } else if ch == u16::from(b'(') && options.has(NumberStyles::ALLOW_PARENTHESES) && (state & STATE_SIGN) == 0 {
                state |= STATE_SIGN | STATE_PARENS;
                number.sign = true;
            } else if let Some(next) = match_opt(s, p, curr_symbol).or_else(|| match_opt(s, p, ansicurr_symbol)) {
                state |= STATE_CURRENCY;
                curr_symbol = None;
                ansicurr_symbol = None;
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
        let hex = options.has(NumberStyles::ALLOW_HEX_SPECIFIER);
        if is_digit(ch) || (hex && (matches!(ch, 0x61..=0x66) || matches!(ch, 0x41..=0x46))) {
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
        } else if let Some(next) = (options.has(NumberStyles::ALLOW_DECIMAL_POINT) && (state & STATE_DECIMAL) == 0)
            .then(|| {
                match_chars(s, p, dec_sep.as_units()).or_else(|| {
                    (parsing_currency && (state & STATE_CURRENCY) == 0)
                        .then(|| match_opt(s, p, altdec_sep))
                        .flatten()
                })
            })
            .flatten()
        {
            state |= STATE_DECIMAL;
            p = next - 1;
        } else if let Some(next) = (options.has(NumberStyles::ALLOW_THOUSANDS)
            && (state & STATE_DIGITS) != 0
            && (state & STATE_DECIMAL) == 0)
            .then(|| {
                match_chars(s, p, group_sep.as_units()).or_else(|| {
                    (parsing_currency && (state & STATE_CURRENCY) == 0)
                        .then(|| match_opt(s, p, altgroup_sep))
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
        if (ch == u16::from(b'E') || ch == u16::from(b'e')) && options.has(NumberStyles::ALLOW_EXPONENT) {
            let temp = p;
            let mut neg_exp = false;
            p += 1;
            ch = at(s, p);
            if let Some(next) = match_chars(s, p, numfmt.positive_sign.as_units()) {
                p = next;
                ch = at(s, p);
            } else if let Some(next) = match_chars(s, p, numfmt.negative_sign.as_units()) {
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
            if is_white(ch) && options.has(NumberStyles::ALLOW_TRAILING_WHITE) {
                // Nothing to do; advance below.
            } else {
                let signflag = options.has(NumberStyles::ALLOW_TRAILING_SIGN) && (state & STATE_SIGN) == 0;
                if let Some(next) = match_chars(s, p, numfmt.positive_sign.as_units()).filter(|_| signflag) {
                    state |= STATE_SIGN;
                    p = next - 1;
                } else if let Some(next) = match_chars(s, p, numfmt.negative_sign.as_units()).filter(|_| signflag) {
                    state |= STATE_SIGN;
                    number.sign = true;
                    p = next - 1;
                } else if ch == u16::from(b')') && (state & STATE_PARENS) != 0 {
                    state &= !STATE_PARENS;
                } else if let Some(next) = match_opt(s, p, curr_symbol).or_else(|| match_opt(s, p, ansicurr_symbol)) {
                    curr_symbol = None;
                    ansicurr_symbol = None;
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
fn trailing_zeros(s: &[u16], index: usize) -> bool {
    s[index..].iter().all(|&c| c == 0)
}

/// `Number.TryStringToNumber`. A missing string (C# `null`) fails.
fn try_string_to_number(
    s: Option<&NetString>,
    options: NumberStyles,
    numfmt: &NumberFormat,
    parse_decimal: bool,
) -> Option<NumberBuffer> {
    let units = s?.as_units();
    let mut number = NumberBuffer::default();
    let (ok, p) = parse_number(units, options, &mut number, numfmt, parse_decimal);
    if !ok || (p < units.len() && !trailing_zeros(units, p)) {
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

/// `Number.HexNumberToUInt32`.
fn hex_number_to_u32(number: &NumberBuffer) -> Option<u32> {
    let mut i = number.scale;
    if i > UINT32_PRECISION || i < number.precision {
        return None;
    }
    let mut digits = number.digits.iter();
    let mut n: u32 = 0;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        if n > (0xFFFF_FFFF / 16) {
            return None;
        }
        n = n.wrapping_mul(16);
        if let Some(&d) = digits.next() {
            let add = match d {
                0x30..=0x39 => u32::from(d - 0x30),
                0x41..=0x46 => u32::from(d - 0x41) + 10,
                _ => u32::from(d - 0x61) + 10,
            };
            let new_n = n.wrapping_add(add);
            if new_n < n {
                return None;
            }
            n = new_n;
        }
    }
    Some(n)
}

/// `Number.TryParseInt32` (what `int.TryParse(s, style, provider, out v)` calls).
pub fn try_parse_i32(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Option<i32> {
    let number = try_string_to_number(s, style, info, false)?;
    if style.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        hex_number_to_u32(&number).map(|v| v as i32)
    } else {
        number_to_i32(&number)
    }
}

/// `Number.TryParseUInt32`.
pub fn try_parse_u32(s: Option<&NetString>, style: NumberStyles, info: &NumberFormat) -> Option<u32> {
    let number = try_string_to_number(s, style, info, false)?;
    if style.has(NumberStyles::ALLOW_HEX_SPECIFIER) {
        hex_number_to_u32(&number)
    } else {
        number_to_u32(&number)
    }
}
