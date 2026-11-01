//! The color helpers: every row of `Assets/Tests/UnturnedDat/UnityDatColorExTests.cs`, one table per C# test
//! method, plus the values a failed parse hands back.

// The tables mirror the C# test methods' parameter lists.
#![allow(clippy::type_complexity)]

mod common;

use common::{found, get_value, parse, tree_with_value};
use pinewatch_dat::{DatTree, ValueRef};
use pinewatch_unity::{Color, Color32};

/// `ValueTryParseColor32RGB`.
const VALUE_TRY_PARSE_COLOR32_R_G_B: &[(&str, bool, u8, u8, u8)] = &[
    ("ff0000", true, 255, 0, 0),
    ("00ff00", true, 0, 255, 0),
    ("0000ff", true, 0, 0, 255),
    ("#ff0000", true, 255, 0, 0),
    ("#00ff00", true, 0, 255, 0),
    ("#0000ff", true, 0, 0, 255),
    ("", false, 0, 0, 0),
    ("#", false, 0, 0, 0),
    ("      ", false, 0, 0, 0),
    ("#      ", false, 0, 0, 0),
    ("zzzzzz", false, 0, 0, 0),
    ("#zzzzzz", false, 0, 0, 0),
];

/// `ValueParseColor32RGB`.
const VALUE_PARSE_COLOR32_R_G_B: &[(&str, u8, u8, u8, u8, u8, u8)] = &[
    ("ff0000", 255, 0, 0, 127, 127, 127),
    ("00ff00", 0, 255, 0, 127, 127, 127),
    ("0000ff", 0, 0, 255, 127, 127, 127),
    ("#ff0000", 255, 0, 0, 127, 127, 127),
    ("#00ff00", 0, 255, 0, 127, 127, 127),
    ("#0000ff", 0, 0, 255, 127, 127, 127),
    ("", 127, 127, 127, 127, 127, 127),
    ("#", 127, 127, 127, 127, 127, 127),
    ("      ", 127, 127, 127, 127, 127, 127),
    ("#      ", 127, 127, 127, 127, 127, 127),
    ("zzzzzz", 127, 127, 127, 127, 127, 127),
    ("#zzzzzz", 127, 127, 127, 127, 127, 127),
];

/// `DictionaryTryParseColor32RGB`.
const DICTIONARY_TRY_PARSE_COLOR32_R_G_B: &[(&str, bool, u8, u8, u8)] = &[
    ("key ff0000", true, 255, 0, 0),
    ("key 00ff00", true, 0, 255, 0),
    ("key 0000ff", true, 0, 0, 255),
    ("key #ff0000", true, 255, 0, 0),
    ("key #00ff00", true, 0, 255, 0),
    ("key #0000ff", true, 0, 0, 255),
    ("key ", false, 0, 0, 0),
    ("key #", false, 0, 0, 0),
    ("key       ", false, 0, 0, 0),
    ("key #      ", false, 0, 0, 0),
    ("key zzzzzz", false, 0, 0, 0),
    ("key #zzzzzz", false, 0, 0, 0),
    ("key\n{\n}", true, 0, 0, 0),
    ("key\n{\nr 1\ng 2\nb 3\n}", true, 1, 2, 3),
];

/// `DictionaryParseColor32RGB`.
const DICTIONARY_PARSE_COLOR32_R_G_B: &[(&str, u8, u8, u8, u8, u8, u8)] = &[
    ("key ff0000", 255, 0, 0, 127, 127, 127),
    ("key 00ff00", 0, 255, 0, 127, 127, 127),
    ("key 0000ff", 0, 0, 255, 127, 127, 127),
    ("key #ff0000", 255, 0, 0, 127, 127, 127),
    ("key #00ff00", 0, 255, 0, 127, 127, 127),
    ("key #0000ff", 0, 0, 255, 127, 127, 127),
    ("key ", 127, 127, 127, 127, 127, 127),
    ("key #", 127, 127, 127, 127, 127, 127),
    ("key       ", 127, 127, 127, 127, 127, 127),
    ("key #      ", 127, 127, 127, 127, 127, 127),
    ("key zzzzzz", 127, 127, 127, 127, 127, 127),
    ("key #zzzzzz", 127, 127, 127, 127, 127, 127),
    ("key\n{\n}", 0, 0, 0, 127, 128, 129),
    ("key\n{\nr 1\ng 2\nb 3\n}", 1, 2, 3, 127, 128, 129),
];

/// `ValueTryParseColor32RGBA`.
const VALUE_TRY_PARSE_COLOR32_R_G_B_A: &[(&str, bool, u8, u8, u8, u8)] = &[
    ("ff0000", true, 255, 0, 0, 255),
    ("00ff00", true, 0, 255, 0, 255),
    ("0000ff", true, 0, 0, 255, 255),
    ("#ff0000", true, 255, 0, 0, 255),
    ("#00ff00", true, 0, 255, 0, 255),
    ("#0000ff", true, 0, 0, 255, 255),
    ("ff00007f", true, 255, 0, 0, 127),
    ("00ff007f", true, 0, 255, 0, 127),
    ("0000ff7f", true, 0, 0, 255, 127),
    ("#ff00007f", true, 255, 0, 0, 127),
    ("#00ff007f", true, 0, 255, 0, 127),
    ("#0000ff7f", true, 0, 0, 255, 127),
    ("", false, 0, 0, 0, 0),
    ("#", false, 0, 0, 0, 0),
    ("        ", false, 0, 0, 0, 0),
    ("#        ", false, 0, 0, 0, 0),
    ("zzzzzz", false, 0, 0, 0, 0),
    ("#zzzzzz", false, 0, 0, 0, 0),
    ("zzzzzzzz", false, 0, 0, 0, 0),
    ("#zzzzzzzz", false, 0, 0, 0, 0),
];

/// `ValueParseColor32RGBA`.
const VALUE_PARSE_COLOR32_R_G_B_A: &[(&str, u8, u8, u8, u8, u8, u8, u8, u8)] = &[
    ("ff0000", 255, 0, 0, 255, 127, 127, 127, 0),
    ("00ff00", 0, 255, 0, 255, 127, 127, 127, 0),
    ("0000ff", 0, 0, 255, 255, 127, 127, 127, 0),
    ("#ff0000", 255, 0, 0, 255, 127, 127, 127, 0),
    ("#00ff00", 0, 255, 0, 255, 127, 127, 127, 0),
    ("#0000ff", 0, 0, 255, 255, 127, 127, 127, 0),
    ("ff00007f", 255, 0, 0, 127, 127, 127, 127, 0),
    ("00ff007f", 0, 255, 0, 127, 127, 127, 127, 0),
    ("0000ff7f", 0, 0, 255, 127, 127, 127, 127, 0),
    ("#ff00007f", 255, 0, 0, 127, 127, 127, 127, 0),
    ("#00ff007f", 0, 255, 0, 127, 127, 127, 127, 0),
    ("#0000ff7f", 0, 0, 255, 127, 127, 127, 127, 0),
    ("", 127, 127, 127, 127, 127, 127, 127, 127),
    ("#", 127, 127, 127, 127, 127, 127, 127, 127),
    ("      ", 127, 127, 127, 127, 127, 127, 127, 127),
    ("#      ", 127, 127, 127, 127, 127, 127, 127, 127),
    ("zzzzzz", 127, 127, 127, 127, 127, 127, 127, 127),
    ("#zzzzzz", 127, 127, 127, 127, 127, 127, 127, 127),
];

/// `DictionaryTryParseColor32RGBA`.
const DICTIONARY_TRY_PARSE_COLOR32_R_G_B_A: &[(&str, bool, u8, u8, u8, u8)] = &[
    ("key ff0000", true, 255, 0, 0, 255),
    ("key 00ff00", true, 0, 255, 0, 255),
    ("key 0000ff", true, 0, 0, 255, 255),
    ("key #ff0000", true, 255, 0, 0, 255),
    ("key #00ff00", true, 0, 255, 0, 255),
    ("key #0000ff", true, 0, 0, 255, 255),
    ("key ff00007f", true, 255, 0, 0, 127),
    ("key 00ff007f", true, 0, 255, 0, 127),
    ("key 0000ff7f", true, 0, 0, 255, 127),
    ("key #ff00007f", true, 255, 0, 0, 127),
    ("key #00ff007f", true, 0, 255, 0, 127),
    ("key #0000ff7f", true, 0, 0, 255, 127),
    ("key ", false, 0, 0, 0, 0),
    ("key #", false, 0, 0, 0, 0),
    ("key         ", false, 0, 0, 0, 0),
    ("key #        ", false, 0, 0, 0, 0),
    ("key zzzzzz", false, 0, 0, 0, 0),
    ("key #zzzzzz", false, 0, 0, 0, 0),
    ("key zzzzzzzz", false, 0, 0, 0, 0),
    ("key #zzzzzzzz", false, 0, 0, 0, 0),
    ("key\n{\n}", true, 0, 0, 0, 0),
    ("key\n{\nr 1\ng 2\nb 3\na 4\n}", true, 1, 2, 3, 4),
];

/// `DictionaryParseColor32RGBA`.
const DICTIONARY_PARSE_COLOR32_R_G_B_A: &[(&str, u8, u8, u8, u8, u8, u8, u8, u8)] = &[
    ("key ff0000", 255, 0, 0, 255, 127, 127, 127, 0),
    ("key 00ff00", 0, 255, 0, 255, 127, 127, 127, 0),
    ("key 0000ff", 0, 0, 255, 255, 127, 127, 127, 0),
    ("key #ff0000", 255, 0, 0, 255, 127, 127, 127, 0),
    ("key #00ff00", 0, 255, 0, 255, 127, 127, 127, 0),
    ("key #0000ff", 0, 0, 255, 255, 127, 127, 127, 0),
    ("key ff00007f", 255, 0, 0, 127, 127, 127, 127, 0),
    ("key 00ff007f", 0, 255, 0, 127, 127, 127, 127, 0),
    ("key 0000ff7f", 0, 0, 255, 127, 127, 127, 127, 0),
    ("key #ff00007f", 255, 0, 0, 127, 127, 127, 127, 0),
    ("key #00ff007f", 0, 255, 0, 127, 127, 127, 127, 0),
    ("key #0000ff7f", 0, 0, 255, 127, 127, 127, 127, 0),
    ("key ", 127, 127, 127, 127, 127, 127, 127, 127),
    ("key #", 127, 127, 127, 127, 127, 127, 127, 127),
    ("key       ", 127, 127, 127, 127, 127, 127, 127, 127),
    ("key #      ", 127, 127, 127, 127, 127, 127, 127, 127),
    ("key zzzzzz", 127, 127, 127, 127, 127, 127, 127, 127),
    ("key #zzzzzz", 127, 127, 127, 127, 127, 127, 127, 127),
    ("key\n{\n}", 0, 0, 0, 0, 127, 128, 129, 130),
    ("key\n{\nr 1\ng 2\nb 3\na 4\n}", 1, 2, 3, 4, 127, 128, 129, 130),
];

/// `LegacyParseColor`.
const LEGACY_PARSE_COLOR: &[(&str, f32, f32, f32, f32, f32, f32)] = &[
    ("key_r 1\nkey_g 0.5\nkey_b 0.25", 1.0, 0.5, 0.25, 0.0, 0.0, 0.0),
    ("key_r a\nkey_g b\nkey_b c", 1.0, 0.5, 0.25, 1.0, 0.5, 0.25),
    ("key #ffffff", 1.0, 1.0, 1.0, 0.0, 0.0, 0.0),
];

/// `LegacyParseColor32RGB`.
const LEGACY_PARSE_COLOR32_R_G_B: &[(&str, u8, u8, u8, u8, u8, u8)] = &[
    ("key_r 1\nkey_g 2\nkey_b 3", 1, 2, 3, 0, 0, 0),
    ("key_r a\nkey_g b\nkey_b c", 1, 2, 3, 1, 2, 3),
    ("key #ff7f3f", 255, 127, 63, 0, 0, 0),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("ValueTryParseColor32RGB", VALUE_TRY_PARSE_COLOR32_R_G_B.len()),
    ("ValueParseColor32RGB", VALUE_PARSE_COLOR32_R_G_B.len()),
    ("DictionaryTryParseColor32RGB", DICTIONARY_TRY_PARSE_COLOR32_R_G_B.len()),
    ("DictionaryParseColor32RGB", DICTIONARY_PARSE_COLOR32_R_G_B.len()),
    ("ValueTryParseColor32RGBA", VALUE_TRY_PARSE_COLOR32_R_G_B_A.len()),
    ("ValueParseColor32RGBA", VALUE_PARSE_COLOR32_R_G_B_A.len()),
    ("DictionaryTryParseColor32RGBA", DICTIONARY_TRY_PARSE_COLOR32_R_G_B_A.len()),
    ("DictionaryParseColor32RGBA", DICTIONARY_PARSE_COLOR32_R_G_B_A.len()),
    ("LegacyParseColor", LEGACY_PARSE_COLOR.len()),
    ("LegacyParseColor32RGB", LEGACY_PARSE_COLOR32_R_G_B.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 138, "{TABLES:?}");
}

/// The node `new DatValue(input)` makes.
fn value(tree: &DatTree) -> ValueRef<'_> {
    found(get_value(tree.root(), "key"), "the value", "key")
}

/// `new Color32(r, g, b, byte.MaxValue)`.
fn opaque(r: u8, g: u8, b: u8) -> Color32 {
    Color32::new(r, g, b, u8::MAX)
}

#[test]
fn value_try_parse_color32_rgb() {
    for &(input, expected_success, expected_r, expected_g, expected_b) in VALUE_TRY_PARSE_COLOR32_R_G_B {
        let tree = tree_with_value(input);
        let (actual_success, actual_value) = value(&tree).try_parse_color32_rgb();
        assert_eq!(expected_success, actual_success, "TryParse result for {input:?}");
        assert_eq!(opaque(expected_r, expected_g, expected_b), actual_value, "for {input:?}");
    }
}

#[test]
fn value_parse_color32_rgb() {
    for &(input, expected_r, expected_g, expected_b, default_r, default_g, default_b) in VALUE_PARSE_COLOR32_R_G_B {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).parse_color32_rgb(opaque(default_r, default_g, default_b));
        assert_eq!(opaque(expected_r, expected_g, expected_b), actual_value, "for {input:?}");
    }
}

#[test]
fn dictionary_try_parse_color32_rgb() {
    for &(input, expected_success, expected_r, expected_g, expected_b) in DICTIONARY_TRY_PARSE_COLOR32_R_G_B {
        let (_, tree) = parse(input);
        let (actual_success, actual_value) = tree.root().try_parse_color32_rgb("key");
        assert_eq!(expected_success, actual_success, "TryParse result for {input:?}");
        assert_eq!(opaque(expected_r, expected_g, expected_b), actual_value, "for {input:?}");
    }
}

#[test]
fn dictionary_parse_color32_rgb() {
    for &(input, expected_r, expected_g, expected_b, default_r, default_g, default_b) in DICTIONARY_PARSE_COLOR32_R_G_B
    {
        let (_, tree) = parse(input);
        let actual_value = tree.root().parse_color32_rgb("key", opaque(default_r, default_g, default_b));
        assert_eq!(opaque(expected_r, expected_g, expected_b), actual_value, "for {input:?}");
    }
}

#[test]
fn value_try_parse_color32_rgba() {
    for &(input, expected_success, expected_r, expected_g, expected_b, expected_a) in VALUE_TRY_PARSE_COLOR32_R_G_B_A
    {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).try_parse_color32_rgba();
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        let expected_value = Color32::new(expected_r, expected_g, expected_b, expected_a);
        assert_eq!(expected_value, actual_value.unwrap_or_default(), "for {input:?}");
    }
}

#[test]
fn value_parse_color32_rgba() {
    for &(input, r, g, b, a, default_r, default_g, default_b, default_a) in VALUE_PARSE_COLOR32_R_G_B_A {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).parse_color32_rgba(Color32::new(default_r, default_g, default_b, default_a));
        assert_eq!(Color32::new(r, g, b, a), actual_value, "for {input:?}");
    }
}

#[test]
fn dictionary_try_parse_color32_rgba() {
    for &(input, expected_success, expected_r, expected_g, expected_b, expected_a) in
        DICTIONARY_TRY_PARSE_COLOR32_R_G_B_A
    {
        let (_, tree) = parse(input);
        let actual_value = tree.root().try_parse_color32_rgba("key");
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        let expected_value = Color32::new(expected_r, expected_g, expected_b, expected_a);
        assert_eq!(expected_value, actual_value.unwrap_or_default(), "for {input:?}");
    }
}

#[test]
fn dictionary_parse_color32_rgba() {
    for &(input, r, g, b, a, default_r, default_g, default_b, default_a) in DICTIONARY_PARSE_COLOR32_R_G_B_A {
        let (_, tree) = parse(input);
        let default_value = Color32::new(default_r, default_g, default_b, default_a);
        let actual_value = tree.root().parse_color32_rgba("key", default_value);
        assert_eq!(Color32::new(r, g, b, a), actual_value, "for {input:?}");
    }
}

#[test]
fn legacy_parse_color() {
    for &(input, expected_r, expected_g, expected_b, default_r, default_g, default_b) in LEGACY_PARSE_COLOR {
        let (_, tree) = parse(input);
        let actual_value = tree.root().legacy_parse_color("key", Color::rgb(default_r, default_g, default_b));
        assert_eq!(Color::rgb(expected_r, expected_g, expected_b), actual_value, "for {input:?}");
    }
}

#[test]
fn legacy_parse_color32_rgb() {
    for &(input, expected_r, expected_g, expected_b, default_r, default_g, default_b) in LEGACY_PARSE_COLOR32_R_G_B {
        let (_, tree) = parse(input);
        let actual_value = tree.root().legacy_parse_color32_rgb("key", opaque(default_r, default_g, default_b));
        assert_eq!(opaque(expected_r, expected_g, expected_b), actual_value, "for {input:?}");
    }
}

/// A failed `TryParseColor32RGB` hands back opaque black and `ParseColor32RGB` makes its default opaque, while the
/// RGBA forms hand back transparent black and keep the default's alpha.
#[test]
fn failed_parses_hand_back_their_own_values() {
    let tree = tree_with_value("nope");
    assert_eq!((false, opaque(0, 0, 0)), value(&tree).try_parse_color32_rgb());
    assert_eq!(opaque(1, 2, 3), value(&tree).parse_color32_rgb(Color32::new(1, 2, 3, 4)));
    assert_eq!(None, value(&tree).try_parse_color32_rgba());
    assert_eq!(Color32::new(1, 2, 3, 4), value(&tree).parse_color32_rgba(Color32::new(1, 2, 3, 4)));

    let (_, tree) = parse("list\n[\n]");
    assert_eq!((false, opaque(0, 0, 0)), tree.root().try_parse_color32_rgb("list"));
    assert_eq!((false, opaque(0, 0, 0)), tree.root().try_parse_color32_rgb("missing"));
    assert_eq!(None, tree.root().try_parse_color32_rgba("list"));
}

/// A dictionary under the key always succeeds, with 0 for a missing or unreadable channel, so its alpha is 0
/// unless `A` is given. `LegacyParseColor` converts through `Color32` and sets alpha to 1.
#[test]
fn dictionary_colors_and_the_legacy_conversion() {
    let (_, tree) = parse("key\n{\nR 255\nG x\nB 128\n}");
    assert_eq!((true, opaque(255, 0, 128)), tree.root().try_parse_color32_rgb("key"));
    assert_eq!(Some(Color32::new(255, 0, 128, 0)), tree.root().try_parse_color32_rgba("key"));
    let legacy = tree.root().legacy_parse_color("key", Color::rgb(0.5, 0.5, 0.5));
    assert_eq!(Color::from(Color32::new(255, 0, 128, 255)), legacy);
    assert_eq!(1.0, legacy.a);
}
