//! `CharUnicodeInfo.GetUnicodeCategory`, `char.GetUnicodeCategory`, `char.IsNumber` and `char.IsLetterOrDigit` for
//! every UTF-16 code unit, as the shipped `mscorlib.dll` computes them (`docs/VERIFICATION.md`, V3).
//!
//! The pinned values come from `reference/checks/chardump.cs` run on the shipped corlib (game 3.26.3.13, 2026-11-03).
//! It prints the checksum of `CharUnicodeInfo.GetUnicodeCategory` itself. The other three are [`checksum`] over its
//! dump: each record's category byte (`char.GetUnicodeCategory`) and its flags 8 (`IsNumber`) and 64
//! (`IsLetterOrDigit`), as 1 or 0. The rows below are the dump's values too. A model of the C# (mono/corefx
//! `5b4edb9b`), run on the source independently of the Rust table, gives the same values.

use pinewatch_mono::chars::UnicodeCategory as Category;
use pinewatch_mono::chars::{
    char_unicode_info_category, get_unicode_category, is_digit, is_letter_or_digit, is_number, is_white_space,
};
use pinewatch_mono::{EnumUnderlying, NetEnum};

/// FNV-1a (64 bits) over a value for every code unit, each as two little-endian bytes (as in `tests/chars.rs`).
fn checksum(map: impl Fn(u16) -> u16) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for c in 0..=u16::MAX {
        for byte in map(c).to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

fn number(category: Category) -> u16 {
    u16::try_from(category.0).unwrap()
}

/// `UnicodeCategory`'s 30 members in .NET's order, `UppercaseLetter = 0` to `OtherNotAssigned = 29` (corefx
/// `Globalization/UnicodeCategory.cs`).
#[test]
fn members_in_dotnet_order() {
    let info = Category::INFO;
    assert_eq!(
        info.names,
        [
            "UppercaseLetter", "LowercaseLetter", "TitlecaseLetter", "ModifierLetter", "OtherLetter", "NonSpacingMark",
            "SpacingCombiningMark", "EnclosingMark", "DecimalDigitNumber", "LetterNumber", "OtherNumber",
            "SpaceSeparator", "LineSeparator", "ParagraphSeparator", "Control", "Format", "Surrogate", "PrivateUse",
            "ConnectorPunctuation", "DashPunctuation", "OpenPunctuation", "ClosePunctuation",
            "InitialQuotePunctuation", "FinalQuotePunctuation", "OtherPunctuation", "MathSymbol", "CurrencySymbol",
            "ModifierSymbol", "OtherSymbol", "OtherNotAssigned",
        ]
    );
    assert_eq!(info.values, (0..30).collect::<Vec<u64>>());
    assert_eq!(info.underlying, EnumUnderlying::I32);
    assert_eq!((Category::UppercaseLetter, Category::OtherNotAssigned), (Category(0), Category(29)));
}

#[test]
fn whole_tables_match_the_shipped_corlib() {
    assert_eq!(checksum(|c| number(char_unicode_info_category(c))), 0x4280_054d_ea7f_bc49);
    assert_eq!(checksum(|c| number(get_unicode_category(c))), 0x5cd4_5c71_2e21_a2e5);
    assert_eq!(checksum(|c| u16::from(is_number(c))), 0x1c7d_69a7_ff0f_4834);
    assert_eq!(checksum(|c| u16::from(is_letter_or_digit(c))), 0xc59a_8dd0_5270_38ed);
    assert_eq!((0..=u16::MAX).filter(|&c| is_number(c)).count(), 735);
    assert_eq!((0..=u16::MAX).filter(|&c| is_letter_or_digit(c)).count(), 49_202);
    // `char.IsDigit`: the dump's flag 4 (its checksum computed from the shipped corlib's dump on 2026-11-05).
    assert_eq!(checksum(|c| u16::from(is_digit(c))), 0xef1f_7f89_9317_dafd);
    assert_eq!((0..=u16::MAX).filter(|&c| is_digit(c)).count(), 370);
    assert!(is_digit(0x0663) && !is_digit(0x00B2) && !is_digit(0x2160));
}

/// What Json.NET's reader asks: `ParseValue` sends a `char.IsNumber` character to the number parser, and unquoted
/// property names take `char.IsLetterOrDigit` characters (plus `_` and `$`, which it tests itself).
#[test]
fn rows_json_net_reads() {
    for (c, category, numeric, letter_or_digit) in [
        (0x0030, Category::DecimalDigitNumber, true, true), // 0
        (0x0039, Category::DecimalDigitNumber, true, true), // 9
        (0x0041, Category::UppercaseLetter, false, true),   // A
        (0x005A, Category::UppercaseLetter, false, true),   // Z
        (0x0061, Category::LowercaseLetter, false, true),   // a
        (0x007A, Category::LowercaseLetter, false, true),   // z
        (0x005F, Category::ConnectorPunctuation, false, false), // _
        (0x0024, Category::CurrencySymbol, false, false),   // $
        (0x002B, Category::MathSymbol, false, false),       // +
        (0x002D, Category::DashPunctuation, false, false),  // -
        (0x002E, Category::OtherPunctuation, false, false), // .
        (0x00B2, Category::OtherNumber, true, false),       // ² SUPERSCRIPT TWO: a number, not a digit
        (0x00BD, Category::OtherNumber, true, false),       // ½
        (0x00B5, Category::LowercaseLetter, false, true),   // µ
        (0x00C0, Category::UppercaseLetter, false, true),   // À
        (0x00FF, Category::LowercaseLetter, false, true),   // ÿ
        (0x01C5, Category::TitlecaseLetter, false, true),   // ǅ
        (0x02B0, Category::ModifierLetter, false, true),    // ʰ
        (0x0391, Category::UppercaseLetter, false, true),   // Α GREEK CAPITAL LETTER ALPHA
        (0x05D0, Category::OtherLetter, false, true),       // א HEBREW LETTER ALEF
        (0x4E00, Category::OtherLetter, false, true),       // 一
        (0x0660, Category::DecimalDigitNumber, true, true), // ٠ ARABIC-INDIC DIGIT ZERO
        (0x0669, Category::DecimalDigitNumber, true, true), // ٩
        (0x0966, Category::DecimalDigitNumber, true, true), // ० DEVANAGARI DIGIT ZERO
        (0xFF10, Category::DecimalDigitNumber, true, true), // ０ FULLWIDTH DIGIT ZERO
        (0x2160, Category::LetterNumber, true, false),      // Ⅰ ROMAN NUMERAL ONE
        (0x3007, Category::LetterNumber, true, false),      // 〇
        (0x0300, Category::NonSpacingMark, false, false),   // COMBINING GRAVE ACCENT
        (0x9FEF, Category::OtherLetter, false, true),       // the last CJK ideograph this data has
        (0x9FF0, Category::OtherNotAssigned, false, false), // a letter only in later Unicode versions
        (0x0378, Category::OtherNotAssigned, false, false), // unassigned
        (0xFDD0, Category::OtherNotAssigned, false, false), // noncharacters
        (0xFFFE, Category::OtherNotAssigned, false, false),
        (0xFFFF, Category::OtherNotAssigned, false, false),
        (0xD800, Category::Surrogate, false, false), // lone surrogates
        (0xDBFF, Category::Surrogate, false, false),
        (0xDC00, Category::Surrogate, false, false),
        (0xDFFF, Category::Surrogate, false, false),
        (0xE000, Category::PrivateUse, false, false),
    ] {
        assert_eq!(get_unicode_category(c), category, "U+{c:04X}");
        assert_eq!(char_unicode_info_category(c), category, "U+{c:04X}");
        assert_eq!(is_number(c), numeric, "U+{c:04X}");
        assert_eq!(is_letter_or_digit(c), letter_or_digit, "U+{c:04X}");
    }
}

/// Up to U+00FF, `char` reads its own table, `s_categoryForLatin1`, which differs from `CharUnicodeInfo`'s data for
/// exactly five code units (`char` from the dump, `CharUnicodeInfo` from the source). `IsNumber` and
/// `IsLetterOrDigit` give the same answers from either table.
#[test]
fn latin1_comes_from_chars_own_table() {
    for (c, from_char, from_info) in [
        (0x00A7, Category::OtherSymbol, Category::OtherPunctuation), // §
        (0x00AA, Category::LowercaseLetter, Category::OtherLetter),  // ª
        (0x00AD, Category::DashPunctuation, Category::Format),       // SOFT HYPHEN
        (0x00B6, Category::OtherSymbol, Category::OtherPunctuation), // ¶
        (0x00BA, Category::LowercaseLetter, Category::OtherLetter),  // º
    ] {
        assert_eq!(get_unicode_category(c), from_char, "U+{c:04X}");
        assert_eq!(char_unicode_info_category(c), from_info, "U+{c:04X}");
    }
    let differing: Vec<u16> =
        (0..=u16::MAX).filter(|&c| get_unicode_category(c) != char_unicode_info_category(c)).collect();
    assert_eq!(differing, [0x00A7, 0x00AA, 0x00AD, 0x00B6, 0x00BA]);
    assert!(is_letter_or_digit(0x00AA) && is_letter_or_digit(0x00BA));
    assert!(!is_number(0x00AA) && !is_letter_or_digit(0x00AD));
}

/// `char.IsWhiteSpace` above Latin-1 is `CharUnicodeInfo.IsWhiteSpace`: the space, line and paragraph separator
/// categories. `chars.rs` lists those code units itself; the category table agrees.
#[test]
fn white_space_above_latin1_is_the_separator_categories() {
    for c in 0x100..=u16::MAX {
        let separator = matches!(
            char_unicode_info_category(c),
            Category::SpaceSeparator | Category::LineSeparator | Category::ParagraphSeparator
        );
        assert_eq!(is_white_space(c), separator, "U+{c:04X}");
    }
}
