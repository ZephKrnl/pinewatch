// Ported from Json.NET 13.0.3 (commit 0a2e291c), `Src/Newtonsoft.Json/Utilities/ConvertUtils.cs`. Copyright (c) 2007
// James Newton-King. MIT license; see NOTICES-PORTED-CODE.md.

//! The integer and hex readers of Json.NET's `ConvertUtils` that the reader uses, and the text of a `BigInteger`.

/// `ParseResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseResult {
    Success,
    Overflow,
    Invalid,
}

fn digit(unit: u16) -> Option<i64> {
    let value = i64::from(unit) - i64::from(b'0');
    (0..=9).contains(&value).then_some(value)
}

/// The shared loop of `Int32TryParse` and `Int64TryParse`: an optional `-`, then ASCII digits only. Digits are
/// accumulated as a negative number so the minimum fits; `max_length` digits or more go straight to the overflow
/// check (`too_long` says when), and an invalid character anywhere wins over overflow.
fn try_parse(chars: &[u16], start: usize, length: usize, min: i64, max_length: usize) -> (ParseResult, i64) {
    if length == 0 {
        return (ParseResult::Invalid, 0);
    }
    let negative = chars[start] == u16::from(b'-');
    let (start, length) = if negative {
        if length == 1 {
            return (ParseResult::Invalid, 0);
        }
        (start + 1, length - 1)
    } else {
        (start, length)
    };
    let digits = &chars[start..start + length];
    let all_digits = || digits.iter().all(|&unit| digit(unit).is_some());
    let too_long = length > max_length
        || (max_length == 10 && length == 10 && i64::from(digits[0]) - i64::from(b'0') > 2);
    if too_long {
        return (if all_digits() { ParseResult::Overflow } else { ParseResult::Invalid }, 0);
    }
    let mut value: i64 = 0;
    for (i, &unit) in digits.iter().enumerate() {
        let Some(d) = digit(unit) else {
            return (ParseResult::Invalid, 0);
        };
        // The C# `10 * value - c` wraps to a positive number exactly when the true value is below the minimum.
        let new_value = 10 * i128::from(value) - i128::from(d);
        if new_value < i128::from(min) {
            // The rest must still be digits for this to be an overflow.
            return (
                if digits[i + 1..].iter().all(|&unit| digit(unit).is_some()) {
                    ParseResult::Overflow
                } else {
                    ParseResult::Invalid
                },
                0,
            );
        }
        value = new_value as i64;
    }
    if !negative {
        if value == min {
            return (ParseResult::Overflow, 0);
        }
        value = -value;
    }
    (ParseResult::Success, value)
}

/// `Int64TryParse(chars, start, length, out value)`.
pub(crate) fn int64_try_parse(chars: &[u16], start: usize, length: usize) -> (ParseResult, i64) {
    try_parse(chars, start, length, i64::MIN, 19)
}

/// `Int32TryParse(chars, start, length, out value)`. Ten digits starting with 3 to 9 count as an overflow without
/// being read.
pub(crate) fn int32_try_parse(chars: &[u16], start: usize, length: usize) -> (ParseResult, i32) {
    let (result, value) = try_parse(chars, start, length, i64::from(i32::MIN), 10);
    (result, value as i32)
}

/// `TryHexTextToInt(text, start, end, out value)`: ASCII hex digits, either case.
pub(crate) fn try_hex_text_to_int(chars: &[u16], start: usize, end: usize) -> Option<i32> {
    let mut value = 0;
    for (i, &unit) in chars[start..end].iter().enumerate() {
        let digit = match unit {
            0x30..=0x39 => i32::from(unit) - 0x30,
            0x41..=0x46 => i32::from(unit) - 0x37,
            0x61..=0x66 => i32::from(unit) - 0x57,
            _ => return None,
        };
        value += digit << ((end - start - 1 - i) * 4);
    }
    Some(value)
}

/// `BigInteger.Parse(number, CultureInfo.InvariantCulture).ToString()` for the text `Int64TryParse` called an
/// overflow, which is an optional `-` and ASCII digits: the digits without leading zeros, and no sign for zero.
pub(crate) fn big_integer_text(number: &[u16]) -> String {
    let negative = number.first() == Some(&u16::from(b'-'));
    let digits: String =
        number[usize::from(negative)..].iter().map(|&unit| char::from(unit as u8)).skip_while(|&c| c == '0').collect();
    match (digits.is_empty(), negative) {
        (true, _) => "0".to_string(),
        (false, true) => format!("-{digits}"),
        (false, false) => digits,
    }
}
