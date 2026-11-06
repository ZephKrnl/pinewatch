// The Latin-1 rules, `GetUnicodeCategory`, `IsNumber` and `IsLetterOrDigit` are ported from corefx
// `src/Common/src/CoreLib/System/Char.cs`, and `CharUnicodeInfo.GetUnicodeCategory` from
// `src/Common/src/CoreLib/System/Globalization/CharUnicodeInfo.cs` (mono/corefx 5b4edb9b). Copyright (c) .NET
// Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.
// The case mapping follows `TextInfo.ToUpper(char)` and `ToLower(char)` in reference source `textinfo.cs` (MONO
// branch) and Mono's `ReferenceSources/TextInfo.cs` (Unity mono b8eb56f). MIT license; see NOTICES-PORTED-CODE.md.

//! `System.Char` classification and case mapping for one UTF-16 code unit, as Unity's Mono computes them.
//!
//! - `IsWhiteSpace` follows corefx: `IsWhiteSpaceLatin1` up to U+00FF, then the space, line and paragraph separator
//!   categories of `CharUnicodeInfoData` (mono/corefx 5b4edb9b), listed here.
//! - `GetUnicodeCategory`, `IsNumber` and `IsLetterOrDigit` follow corefx too: `char`'s own table up to U+00FF, then
//!   `CharUnicodeInfo.GetUnicodeCategory`, whose results `chars/category_table.rs` lists. That is corefx's Unicode
//!   data, not Rust's: Rust's `char` methods know later Unicode versions.
//! - `ToUpperInvariant` and `ToLowerInvariant` are `CultureInfo.InvariantCulture.TextInfo.ToUpper` and `ToLower`:
//!   ASCII is folded directly, everything else comes from Mono's tables (`case_tables.rs`).
//!
//! These are ported from source, and the shipped `mscorlib.dll` gives the same results for every code unit
//! (`docs/VERIFICATION.md`, V3).

mod category_table;

use crate::case_tables::{TO_LOWER, TO_UPPER};

mod unicode_category {
    #![allow(non_upper_case_globals)] // .NET's member names, which `Enum.TryParse` matches

    crate::net_enum! {
        /// `System.Globalization.UnicodeCategory` (corefx `Globalization/UnicodeCategory.cs`).
        pub struct UnicodeCategory(i32) {
            UppercaseLetter = 0,
            LowercaseLetter = 1,
            TitlecaseLetter = 2,
            ModifierLetter = 3,
            OtherLetter = 4,
            NonSpacingMark = 5,
            SpacingCombiningMark = 6,
            EnclosingMark = 7,
            DecimalDigitNumber = 8,
            LetterNumber = 9,
            OtherNumber = 10,
            SpaceSeparator = 11,
            LineSeparator = 12,
            ParagraphSeparator = 13,
            Control = 14,
            Format = 15,
            Surrogate = 16,
            PrivateUse = 17,
            ConnectorPunctuation = 18,
            DashPunctuation = 19,
            OpenPunctuation = 20,
            ClosePunctuation = 21,
            InitialQuotePunctuation = 22,
            FinalQuotePunctuation = 23,
            OtherPunctuation = 24,
            MathSymbol = 25,
            CurrencySymbol = 26,
            ModifierSymbol = 27,
            OtherSymbol = 28,
            OtherNotAssigned = 29,
        }
    }
}

pub use unicode_category::UnicodeCategory;

/// `char.IsWhiteSpace`.
pub fn is_white_space(c: u16) -> bool {
    if c < 0x100 {
        // `IsWhiteSpaceLatin1`.
        c == 0x20 || (0x09..=0x0D).contains(&c) || c == 0xA0 || c == 0x85
    } else {
        // `CharUnicodeInfo.IsWhiteSpace`: categories SpaceSeparator, LineSeparator and ParagraphSeparator.
        matches!(c, 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
    }
}

/// `CharUnicodeInfo.GetUnicodeCategory(char)`: the category in corefx's Unicode data (`CharUnicodeInfoData`). A
/// lone surrogate is `Surrogate`; an unassigned code unit or a noncharacter is `OtherNotAssigned`.
pub fn char_unicode_info_category(c: u16) -> UnicodeCategory {
    let runs = &category_table::CATEGORY_RUNS;
    // The first run starts at U+0000, so `c` is in the last run that starts at or before it.
    let run = runs.partition_point(|&(start, _)| start <= c) - 1;
    UnicodeCategory(i32::from(runs[run].1))
}

/// `GetLatin1UnicodeCategory`: `char`'s own table, `s_categoryForLatin1`, for U+0000 to U+00FF. It differs from
/// `CharUnicodeInfo` for five code units: `§` and `¶` are `OtherSymbol` here, `ª` and `º` `LowercaseLetter`, and the
/// soft hyphen U+00AD `DashPunctuation`.
fn latin1_category(c: u16) -> UnicodeCategory {
    UnicodeCategory(i32::from(category_table::CATEGORY_FOR_LATIN1[usize::from(c)]))
}

/// `char.GetUnicodeCategory(char)`: `char`'s own table up to U+00FF, then `CharUnicodeInfo.GetUnicodeCategory`.
pub fn get_unicode_category(c: u16) -> UnicodeCategory {
    if c < 0x100 { latin1_category(c) } else { char_unicode_info_category(c) }
}

/// `CheckNumber`.
fn check_number(category: UnicodeCategory) -> bool {
    matches!(
        category,
        UnicodeCategory::DecimalDigitNumber | UnicodeCategory::LetterNumber | UnicodeCategory::OtherNumber
    )
}

/// `CheckLetterOrDigit`.
fn check_letter_or_digit(category: UnicodeCategory) -> bool {
    matches!(
        category,
        UnicodeCategory::UppercaseLetter
            | UnicodeCategory::LowercaseLetter
            | UnicodeCategory::TitlecaseLetter
            | UnicodeCategory::ModifierLetter
            | UnicodeCategory::OtherLetter
            | UnicodeCategory::DecimalDigitNumber
    )
}

/// `char.IsNumber`: an ASCII digit, or any character in a number category (`DecimalDigitNumber`, `LetterNumber`,
/// `OtherNumber`), so `²`, `½` and `Ⅰ` are numbers too.
pub fn is_number(c: u16) -> bool {
    if c < 0x80 {
        // `IsAscii`.
        (0x30..=0x39).contains(&c)
    } else if c < 0x100 {
        check_number(latin1_category(c))
    } else {
        check_number(char_unicode_info_category(c))
    }
}

/// `char.IsDigit`: `0` to `9` in Latin-1, any `DecimalDigitNumber` above it (Arabic-Indic digits, for example).
pub fn is_digit(c: u16) -> bool {
    if c < 0x100 {
        (0x30..=0x39).contains(&c)
    } else {
        char_unicode_info_category(c) == UnicodeCategory::DecimalDigitNumber
    }
}

/// `char.IsLetterOrDigit`: any character in a letter category or `DecimalDigitNumber`.
pub fn is_letter_or_digit(c: u16) -> bool {
    // `IsLatin1(c) ? GetLatin1UnicodeCategory(c) : CharUnicodeInfo.GetUnicodeCategory(c)`, which is
    // `GetUnicodeCategory`.
    check_letter_or_digit(get_unicode_category(c))
}

/// The table's mapping for `c`, or `c` itself.
fn mapped(table: &[(u16, u16)], c: u16) -> u16 {
    table.binary_search_by_key(&c, |&(from, _)| from).map_or(c, |index| table[index].1)
}

/// `char.ToLowerInvariant`: `TextInfo.ToLower(char)` for the invariant culture.
pub fn to_lower_invariant(c: u16) -> u16 {
    if c < 0x80 {
        // `ToLowerAsciiInvariant`.
        if (0x41..=0x5A).contains(&c) { c + 0x20 } else { c }
    } else {
        mapped(&TO_LOWER, c)
    }
}

/// `char.ToUpperInvariant`: `TextInfo.ToUpper(char)` for the invariant culture.
pub fn to_upper_invariant(c: u16) -> u16 {
    if c < 0x80 {
        // `ToUpperAsciiInvariant`.
        if (0x61..=0x7A).contains(&c) { c - 0x20 } else { c }
    } else {
        mapped(&TO_UPPER, c)
    }
}

/// `StringComparison.OrdinalIgnoreCase` equality: the same length, and every code unit equal after
/// `ToUpperInvariant`.
pub fn equals_ordinal_ignore_case(a: &[u16], b: &[u16]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| x == y || to_upper_invariant(x) == to_upper_invariant(y))
}

/// `string.Trim()` and `ReadOnlySpan<char>.Trim()`: removes `char.IsWhiteSpace` characters from both ends.
pub fn trim(s: &[u16]) -> &[u16] {
    let start = s.iter().position(|&c| !is_white_space(c)).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| !is_white_space(c)).map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}
