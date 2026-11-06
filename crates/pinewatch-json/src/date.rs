// Ported from Json.NET 13.0.3 (commit 0a2e291c), `Src/Newtonsoft.Json/Utilities/DateTimeUtils.cs` and
// `DateTimeParser.cs`. Copyright (c) 2007 James Newton-King. MIT license; see NOTICES-PORTED-CODE.md.
// `DateTime`'s tick arithmetic and its two exceptions follow corefx `src/Common/src/CoreLib/System/DateTime.cs`
// (mono/corefx 5b4edb9b). Copyright (c) .NET Foundation and Contributors. MIT license.

//! `DateTimeUtils.TryParseDateTime` with `DateTimeZoneHandling.RoundtripKind` and no `DateFormatString`: ISO 8601
//! text (`DateTimeParser`) and Microsoft's `/Date(ticks)/`.
//!
//! A date with an offset is converted to the local time zone in the original. Time zones aren't ported yet, so the
//! local zone is taken to be UTC: the date holds the UTC instant, marked `Local` (`docs/VERIFICATION.md`, V31).

use pinewatch_mono::{ExceptionKind, MonoException, NEW_LINE, chars, sr};

use crate::number::{ParseResult, int32_try_parse, int64_try_parse};

mod kind {
    #![allow(non_upper_case_globals)] // .NET's member names

    pinewatch_mono::net_enum! {
        /// `System.DateTimeKind`.
        pub struct DateTimeKind(i32) {
            Unspecified = 0,
            Utc = 1,
            Local = 2,
        }
    }
}

pub use kind::DateTimeKind;

/// A `DateTime`: its ticks (100 ns since 0001-01-01) and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonDate {
    pub ticks: i64,
    pub kind: DateTimeKind,
}

const TICKS_PER_SECOND: i64 = 10_000_000;
const TICKS_PER_DAY: i64 = 864_000_000_000;
const MAX_TICKS: i64 = 3_155_378_975_999_999_999;
const INITIAL_JAVASCRIPT_DATE_TICKS: i64 = 621_355_968_000_000_000;
const DAYS_TO_MONTH_365: [i64; 13] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];
const DAYS_TO_MONTH_366: [i64; 13] = [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366];

fn days_to_month(year: i32) -> &'static [i64; 13] {
    if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { &DAYS_TO_MONTH_366 } else { &DAYS_TO_MONTH_365 }
}

/// `new ArgumentOutOfRangeException(param, message)`.
fn out_of_range(param: &str, message: &str) -> MonoException {
    MonoException::new(ExceptionKind::ArgumentOutOfRange, format!(
        "{message}{NEW_LINE}{}",
        sr::ARG_PARAM_NAME_NAME.replace("{0}", param)
    ))
}

/// `new DateTime(ticks, kind)`, which checks the range.
fn date_time(ticks: i64, kind: DateTimeKind) -> Result<JsonDate, MonoException> {
    if !(0..=MAX_TICKS).contains(&ticks) {
        return Err(out_of_range("ticks", sr::ARGUMENT_OUT_OF_RANGE_DATE_TIME_BAD_TICKS));
    }
    Ok(JsonDate { ticks, kind })
}

/// `DateTime.AddTicks(value)`.
fn add_ticks(ticks: i64, value: i64) -> Result<i64, MonoException> {
    if value > MAX_TICKS - ticks || value < -ticks {
        return Err(out_of_range("value", sr::ARGUMENT_OUT_OF_RANGE_DATE_ARITHMETIC));
    }
    Ok(ticks + value)
}

/// `ParserTimeZone`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Zone {
    #[default]
    Unspecified,
    Utc,
    LocalWestOfUtc,
    LocalEastOfUtc,
}

const MAX_FRACTION_DIGITS: usize = 7;
const POWER_10: [i32; 7] = [-1, 10, 100, 1000, 10_000, 100_000, 1_000_000];

/// `DateTimeParser`: `yyyy-MM-ddTHH:mm:ss`, then up to 7 fraction digits after `.`, then an optional zone (`Z`, `z`,
/// `+hh` or `-hh`, optionally with `mm` or `:mm`), and nothing after. Minutes are accepted even without a zone sign.
#[derive(Default)]
struct DateTimeParser<'a> {
    text: &'a [u16],
    end: usize,
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    fraction: i32,
    zone_hour: i32,
    zone_minute: i32,
    zone: Zone,
}

impl<'a> DateTimeParser<'a> {
    fn parse(text: &'a [u16], start: usize, length: usize) -> Option<Self> {
        let mut parser = Self { text, end: start + length, ..Self::default() };
        (parser.parse_date(start) && parser.parse_char(start + 10, b'T') && parser.parse_time_and_zone(start + 11))
            .then_some(parser)
    }

    fn digit(&self, at: usize) -> Option<i32> {
        let value = i32::from(self.text[at]) - i32::from(b'0');
        (0..10).contains(&value).then_some(value)
    }

    fn parse_4_digit(&self, start: usize) -> Option<i32> {
        if start + 3 < self.end {
            let digits = [self.digit(start)?, self.digit(start + 1)?, self.digit(start + 2)?, self.digit(start + 3)?];
            return Some(digits.iter().fold(0, |value, digit| value * 10 + digit));
        }
        None
    }

    fn parse_2_digit(&self, start: usize) -> Option<i32> {
        if start + 1 < self.end {
            return Some(self.digit(start)? * 10 + self.digit(start + 1)?);
        }
        None
    }

    fn parse_char(&self, start: usize, ch: u8) -> bool {
        start < self.end && self.text[start] == u16::from(ch)
    }

    fn parse_date(&mut self, start: usize) -> bool {
        let Some(year) = self.parse_4_digit(start).filter(|&year| year >= 1) else { return false };
        self.year = year;
        if !self.parse_char(start + 4, b'-') {
            return false;
        }
        let Some(month) = self.parse_2_digit(start + 5).filter(|month| (1..=12).contains(month)) else { return false };
        self.month = month;
        if !self.parse_char(start + 7, b'-') {
            return false;
        }
        let table = days_to_month(year);
        let days_in_month = (table[month as usize] - table[month as usize - 1]) as i32;
        let Some(day) = self.parse_2_digit(start + 8).filter(|day| (1..=days_in_month).contains(day)) else {
            return false;
        };
        self.day = day;
        true
    }

    fn parse_time_and_zone(&mut self, mut start: usize) -> bool {
        self.parse_time(&mut start) && self.parse_zone(start)
    }

    fn parse_time(&mut self, start: &mut usize) -> bool {
        let Some(hour) = self.parse_2_digit(*start).filter(|&hour| hour <= 24) else { return false };
        self.hour = hour;
        if !self.parse_char(*start + 2, b':') {
            return false;
        }
        let Some(minute) = self.parse_2_digit(*start + 3).filter(|&minute| minute < 60) else { return false };
        self.minute = minute;
        if !self.parse_char(*start + 5, b':') {
            return false;
        }
        let Some(second) = self.parse_2_digit(*start + 6).filter(|&second| second < 60) else { return false };
        self.second = second;
        if hour == 24 && (minute != 0 || second != 0) {
            return false;
        }
        *start += 8;
        if self.parse_char(*start, b'.') {
            self.fraction = 0;
            let mut digits = 0;
            loop {
                *start += 1;
                if *start >= self.end || digits >= MAX_FRACTION_DIGITS {
                    break;
                }
                let Some(digit) = self.digit(*start) else { break };
                self.fraction = self.fraction * 10 + digit;
                digits += 1;
            }
            if digits < MAX_FRACTION_DIGITS {
                if digits == 0 {
                    return false;
                }
                self.fraction *= POWER_10[MAX_FRACTION_DIGITS - digits];
            }
            if self.hour == 24 && self.fraction != 0 {
                return false;
            }
        }
        true
    }

    fn parse_zone(&mut self, mut start: usize) -> bool {
        if start < self.end {
            let ch = self.text[start];
            if ch == u16::from(b'Z') || ch == u16::from(b'z') {
                self.zone = Zone::Utc;
                start += 1;
            } else {
                if start + 2 < self.end
                    && let Some(hour) = self.parse_2_digit(start + 1)
                {
                    self.zone_hour = hour;
                    if ch == u16::from(b'-') {
                        self.zone = Zone::LocalWestOfUtc;
                        start += 3;
                    } else if ch == u16::from(b'+') {
                        self.zone = Zone::LocalEastOfUtc;
                        start += 3;
                    }
                }
                if start < self.end {
                    if self.parse_char(start, b':') {
                        start += 1;
                    }
                    if start + 1 < self.end
                        && let Some(minute) = self.parse_2_digit(start)
                    {
                        self.zone_minute = minute;
                        start += 2;
                    }
                }
            }
        }
        start == self.end
    }

    /// `CreateDateTime`: `new DateTime(year, month, day, hour, minute, second)`, plus the fraction, plus a day for
    /// 24:00 (which `AddDays` refuses past 9999-12-31).
    fn create_date_time(&self) -> Result<i64, MonoException> {
        let hour = if self.hour == 24 { 0 } else { self.hour };
        let y = i64::from(self.year - 1);
        let days = y * 365 + y / 4 - y / 100 + y / 400 + days_to_month(self.year)[self.month as usize - 1]
            + i64::from(self.day)
            - 1;
        let seconds = i64::from(hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second);
        let mut ticks = add_ticks(days * TICKS_PER_DAY + seconds * TICKS_PER_SECOND, i64::from(self.fraction))?;
        if self.hour == 24 {
            ticks = add_ticks(ticks, TICKS_PER_DAY)?;
        }
        Ok(ticks)
    }
}

/// `TryParseDateTimeIso`.
fn try_parse_iso(chars: &[u16], start: usize, length: usize) -> Result<Option<JsonDate>, MonoException> {
    let Some(parser) = DateTimeParser::parse(chars, start, length) else { return Ok(None) };
    let d = parser.create_date_time()?;
    let offset = (i64::from(parser.zone_hour) * 3600 + i64::from(parser.zone_minute) * 60) * TICKS_PER_SECOND;
    // The local zone's offset (`d.GetUtcOffset()`), with the local zone taken to be UTC.
    let local_offset = 0;
    Ok(Some(match parser.zone {
        Zone::Unspecified => JsonDate { ticks: d, kind: DateTimeKind::Unspecified },
        Zone::Utc => JsonDate { ticks: d, kind: DateTimeKind::Utc },
        Zone::LocalWestOfUtc => {
            let ticks = d + offset;
            if ticks <= MAX_TICKS {
                // `new DateTime(ticks, Utc).ToLocalTime()`.
                JsonDate { ticks, kind: DateTimeKind::Local }
            } else {
                JsonDate { ticks: (ticks + local_offset).min(MAX_TICKS), kind: DateTimeKind::Local }
            }
        }
        Zone::LocalEastOfUtc => {
            let ticks = d - offset;
            if ticks >= 0 {
                JsonDate { ticks, kind: DateTimeKind::Local }
            } else {
                JsonDate { ticks: (ticks + local_offset).max(0), kind: DateTimeKind::Local }
            }
        }
    }))
}

/// `TryReadOffset`. `start` is an index into `chars`, but the minutes check compares it with the reference's length,
/// as the original does: for text that sits past the start of the reader's buffer, the minutes aren't read.
fn try_read_offset(chars: &[u16], length: usize, start: usize) -> bool {
    if int32_try_parse(chars, start + 1, 2).0 != ParseResult::Success {
        return false;
    }
    if length as i64 - start as i64 > 5 && int32_try_parse(chars, start + 3, 2).0 != ParseResult::Success {
        return false;
    }
    true
}

/// `TryParseDateTimeMicrosoft`: `/Date(ticks)/` or `/Date(ticks±hhmm)/`, in JavaScript milliseconds since 1970.
fn try_parse_microsoft(chars: &[u16], start: usize, length: usize) -> Result<Option<JsonDate>, MonoException> {
    // `text.IndexOf(c, 7, text.Length - 8)`, relative to the reference.
    let search =
        |c: u8| chars[start + 7..start + length - 1].iter().position(|&unit| unit == u16::from(c)).map(|i| i + 7);
    let (kind, index) = match search(b'+').or_else(|| search(b'-')) {
        Some(index) => {
            if !try_read_offset(chars, length, index + start) {
                return Ok(None);
            }
            (DateTimeKind::Local, index)
        }
        None => (DateTimeKind::Utc, length - 2),
    };
    let (result, javascript_ticks) = int64_try_parse(chars, start + 6, index - 6);
    if result != ParseResult::Success {
        return Ok(None);
    }
    // `ConvertJavaScriptTicksToDateTime`: unchecked arithmetic, then the constructor's range check.
    let ticks = javascript_ticks.wrapping_mul(10_000).wrapping_add(INITIAL_JAVASCRIPT_DATE_TICKS);
    let utc = date_time(ticks, DateTimeKind::Utc)?;
    Ok(Some(JsonDate { ticks: utc.ticks, kind }))
}

fn units_equal(chars: &[u16], at: usize, text: &str) -> bool {
    text.bytes().enumerate().all(|(i, b)| chars[at + i] == u16::from(b))
}

/// `DateTimeUtils.TryParseDateTime(s, DateTimeZoneHandling.RoundtripKind, null, culture, out dt)` for the
/// reference `chars[start..start + length]`. `Err` is an exception thrown out of the parse.
pub(crate) fn try_parse_date_time(
    chars: &[u16],
    start: usize,
    length: usize,
) -> Result<Option<JsonDate>, MonoException> {
    if length > 0 {
        if chars[start] == u16::from(b'/') {
            if length >= 9
                && units_equal(chars, start, "/Date(")
                && units_equal(chars, start + length - 2, ")/")
                && let Some(date) = try_parse_microsoft(chars, start, length)?
            {
                return Ok(Some(date));
            }
        } else if (19..=40).contains(&length)
            && chars::is_digit(chars[start])
            && chars[start + 10] == u16::from(b'T')
            && let Some(date) = try_parse_iso(chars, start, length)?
        {
            return Ok(Some(date));
        }
    }
    Ok(None)
}
