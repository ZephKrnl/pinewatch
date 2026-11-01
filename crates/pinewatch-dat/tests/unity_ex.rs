//! The vector helpers: every row of `Assets/Tests/UnturnedDat/UnityDatExTests.cs`, one table per C# test method,
//! plus the `IndexOf` exception a `(` at the end of the value throws.

mod common;

use common::{found, get_value, parse, tree_with_value};
use pinewatch_dat::{DatTree, ValueRef};
use pinewatch_mono::{MonoException, sr};
use pinewatch_unity::{Vector2, Vector3};

/// `ValueTryParseVector2`.
const VALUE_TRY_PARSE_VECTOR2: &[(&str, bool, f32, f32)] = &[
    ("0,0", true, 0.0, 0.0),
    ("1, 2", true, 1.0, 2.0),
    ("(1, 2)", true, 1.0, 2.0),
    ("0,a", false, 0.0, 0.0),
    ("a,0", false, 0.0, 0.0),
    ("0,", false, 0.0, 0.0),
    (",0", false, 0.0, 0.0),
    (",", false, 0.0, 0.0),
    ("0,", false, 0.0, 0.0),
    ("(0,0", false, 0.0, 0.0),
];

/// `ValueParseVector2`.
const VALUE_PARSE_VECTOR2: &[(&str, f32, f32, f32, f32)] = &[
    ("0,0", 0.0, 0.0, 1.0, 2.0),
    ("1, 2", 1.0, 2.0, 0.0, 0.0),
    ("(1, 2)", 1.0, 2.0, 0.0, 0.0),
    ("a,", 1.0, 2.0, 1.0, 2.0),
    (",a", 1.0, 2.0, 1.0, 2.0),
    ("0,", 1.0, 2.0, 1.0, 2.0),
    (",0", 1.0, 2.0, 1.0, 2.0),
    (",", 1.0, 2.0, 1.0, 2.0),
    ("(0,0", 1.0, 2.0, 1.0, 2.0),
];

/// `DictionaryTryParseVector2`.
const DICTIONARY_TRY_PARSE_VECTOR2: &[(&str, bool, f32, f32)] = &[
    ("key 0,0", true, 0.0, 0.0),
    ("key 1, 2", true, 1.0, 2.0),
    ("key (1, 2)", true, 1.0, 2.0),
    ("key 0,a", false, 0.0, 0.0),
    ("key ,a", false, 0.0, 0.0),
    ("key ,", false, 0.0, 0.0),
    ("key (0,0", false, 0.0, 0.0),
    ("key\n{\n}", true, 0.0, 0.0),
    ("key\n{\nx 1\ny 2\n}", true, 1.0, 2.0),
];

/// `DictionaryParseVector2`.
const DICTIONARY_PARSE_VECTOR2: &[(&str, f32, f32, f32, f32)] = &[
    ("key 0,0", 0.0, 0.0, 1.0, 2.0),
    ("key 1, 2", 1.0, 2.0, 0.0, 0.0),
    ("key (1, 2)", 1.0, 2.0, 0.0, 0.0),
    ("key 0,", 1.0, 2.0, 1.0, 2.0),
    ("key ,", 1.0, 2.0, 1.0, 2.0),
    ("key (0,0", 1.0, 2.0, 1.0, 2.0),
    ("key\n{\n}", 0.0, 0.0, 1.0, 2.0),
    ("key\n{\nx 1\ny 2\n}", 1.0, 2.0, 0.0, 0.0),
];

/// `ValueTryParseVector3`.
const VALUE_TRY_PARSE_VECTOR3: &[(&str, bool, f32, f32, f32)] = &[
    ("0,0,0", true, 0.0, 0.0, 0.0),
    ("1, 2, 3", true, 1.0, 2.0, 3.0),
    ("(1, 2, 3)", true, 1.0, 2.0, 3.0),
    ("0,0,a", false, 0.0, 0.0, 0.0),
    ("0,a,0", false, 0.0, 0.0, 0.0),
    ("a,0,0", false, 0.0, 0.0, 0.0),
    ("0,0,", false, 0.0, 0.0, 0.0),
    ("0,,0", false, 0.0, 0.0, 0.0),
    (",0,0", false, 0.0, 0.0, 0.0),
    (",,0", false, 0.0, 0.0, 0.0),
    ("0,,", false, 0.0, 0.0, 0.0),
    (",0,", false, 0.0, 0.0, 0.0),
    (",,", false, 0.0, 0.0, 0.0),
    ("0,", false, 0.0, 0.0, 0.0),
    (",", false, 0.0, 0.0, 0.0),
    ("(0,0,0", false, 0.0, 0.0, 0.0),
];

/// `ValueParseVector3`.
const VALUE_PARSE_VECTOR3: &[(&str, f32, f32, f32, f32, f32, f32)] = &[
    ("0,0,0", 0.0, 0.0, 0.0, 1.0, 2.0, 3.0),
    ("1, 2, 3", 1.0, 2.0, 3.0, 0.0, 0.0, 0.0),
    ("(1, 2, 3)", 1.0, 2.0, 3.0, 0.0, 0.0, 0.0),
    ("0,0,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("0,,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    (",0,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    (",,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("0,,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    (",0,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    (",,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("0,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    (",", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("(0,0,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
];

/// `DictionaryTryParseVector3`.
const DICTIONARY_TRY_PARSE_VECTOR3: &[(&str, bool, f32, f32, f32)] = &[
    ("key 0,0,0", true, 0.0, 0.0, 0.0),
    ("key 1, 2, 3", true, 1.0, 2.0, 3.0),
    ("key (1, 2, 3)", true, 1.0, 2.0, 3.0),
    ("key 0,0,a", false, 0.0, 0.0, 0.0),
    ("key 0,a,0", false, 0.0, 0.0, 0.0),
    ("key a,0,0", false, 0.0, 0.0, 0.0),
    ("key 0,0,", false, 0.0, 0.0, 0.0),
    ("key 0,,0", false, 0.0, 0.0, 0.0),
    ("key ,0,0", false, 0.0, 0.0, 0.0),
    ("key ,,0", false, 0.0, 0.0, 0.0),
    ("key 0,,", false, 0.0, 0.0, 0.0),
    ("key ,0,", false, 0.0, 0.0, 0.0),
    ("key ,,", false, 0.0, 0.0, 0.0),
    ("key 0,", false, 0.0, 0.0, 0.0),
    ("key ,", false, 0.0, 0.0, 0.0),
    ("key (0,0,0,", false, 0.0, 0.0, 0.0),
    ("key\n{\n}", true, 0.0, 0.0, 0.0),
    ("key\n{\nx 1\ny 2\nz 3\n}", true, 1.0, 2.0, 3.0),
];

/// `DictionaryParseVector3`.
const DICTIONARY_PARSE_VECTOR3: &[(&str, f32, f32, f32, f32, f32, f32)] = &[
    ("key 0,0,0", 0.0, 0.0, 0.0, 1.0, 2.0, 3.0),
    ("key 1, 2, 3", 1.0, 2.0, 3.0, 0.0, 0.0, 0.0),
    ("key (1, 2, 3)", 1.0, 2.0, 3.0, 0.0, 0.0, 0.0),
    ("key 0,0,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key 0,,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key ,0,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key ,,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key 0,,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key ,0,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key ,,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key 0,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key ,", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key (0,0,0", 1.0, 2.0, 3.0, 1.0, 2.0, 3.0),
    ("key\n{\n}", 0.0, 0.0, 0.0, 1.0, 2.0, 3.0),
    ("key\n{\nx 1\ny 2\nz 3\n}", 1.0, 2.0, 3.0, 0.0, 0.0, 0.0),
];

/// `LegacyParseVector3`.
const LEGACY_PARSE_VECTOR3: &[(&str, f32, f32, f32)] = &[
    ("key_x 1\nkey_y 2\nkey_z 3", 1.0, 2.0, 3.0),
    ("key_x a\nkey_y b\nkey_z c", 0.0, 0.0, 0.0),
    ("key (1,2,3)", 1.0, 2.0, 3.0),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("ValueTryParseVector2", VALUE_TRY_PARSE_VECTOR2.len()),
    ("ValueParseVector2", VALUE_PARSE_VECTOR2.len()),
    ("DictionaryTryParseVector2", DICTIONARY_TRY_PARSE_VECTOR2.len()),
    ("DictionaryParseVector2", DICTIONARY_PARSE_VECTOR2.len()),
    ("ValueTryParseVector3", VALUE_TRY_PARSE_VECTOR3.len()),
    ("ValueParseVector3", VALUE_PARSE_VECTOR3.len()),
    ("DictionaryTryParseVector3", DICTIONARY_TRY_PARSE_VECTOR3.len()),
    ("DictionaryParseVector3", DICTIONARY_PARSE_VECTOR3.len()),
    ("LegacyParseVector3", LEGACY_PARSE_VECTOR3.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 101, "{TABLES:?}");
}

/// The node `new DatValue(input)` makes.
fn value(tree: &DatTree) -> ValueRef<'_> {
    found(get_value(tree.root(), "key"), "the value", "key")
}

#[test]
fn value_try_parse_vector2() {
    for &(input, expected_success, expected_x, expected_y) in VALUE_TRY_PARSE_VECTOR2 {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).try_parse_vector2().expect("no exception");
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        assert_eq!(Vector2::new(expected_x, expected_y), actual_value.unwrap_or_default(), "for {input:?}");
    }
}

#[test]
fn value_parse_vector2() {
    for &(input, expected_x, expected_y, default_x, default_y) in VALUE_PARSE_VECTOR2 {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).parse_vector2(Vector2::new(default_x, default_y)).expect("no exception");
        assert_eq!(Vector2::new(expected_x, expected_y), actual_value, "for {input:?}");
    }
}

#[test]
fn dictionary_try_parse_vector2() {
    for &(input, expected_success, expected_x, expected_y) in DICTIONARY_TRY_PARSE_VECTOR2 {
        let (_, tree) = parse(input);
        let actual_value = tree.root().try_parse_vector2("key").expect("no exception");
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        assert_eq!(Vector2::new(expected_x, expected_y), actual_value.unwrap_or_default(), "for {input:?}");
    }
}

#[test]
fn dictionary_parse_vector2() {
    for &(input, expected_x, expected_y, default_x, default_y) in DICTIONARY_PARSE_VECTOR2 {
        let (_, tree) = parse(input);
        let actual_value = tree.root().parse_vector2("key", Vector2::new(default_x, default_y)).expect("no exception");
        assert_eq!(Vector2::new(expected_x, expected_y), actual_value, "for {input:?}");
    }
}

#[test]
fn value_try_parse_vector3() {
    for &(input, expected_success, expected_x, expected_y, expected_z) in VALUE_TRY_PARSE_VECTOR3 {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).try_parse_vector3().expect("no exception");
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        let expected_value = Vector3::new(expected_x, expected_y, expected_z);
        assert_eq!(expected_value, actual_value.unwrap_or_default(), "for {input:?}");
    }
}

#[test]
fn value_parse_vector3() {
    for &(input, expected_x, expected_y, expected_z, default_x, default_y, default_z) in VALUE_PARSE_VECTOR3 {
        let tree = tree_with_value(input);
        let default_value = Vector3::new(default_x, default_y, default_z);
        let actual_value = value(&tree).parse_vector3(default_value).expect("no exception");
        assert_eq!(Vector3::new(expected_x, expected_y, expected_z), actual_value, "for {input:?}");
    }
}

#[test]
fn dictionary_try_parse_vector3() {
    for &(input, expected_success, expected_x, expected_y, expected_z) in DICTIONARY_TRY_PARSE_VECTOR3 {
        let (_, tree) = parse(input);
        let actual_value = tree.root().try_parse_vector3("key").expect("no exception");
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        let expected_value = Vector3::new(expected_x, expected_y, expected_z);
        assert_eq!(expected_value, actual_value.unwrap_or_default(), "for {input:?}");
    }
}

#[test]
fn dictionary_parse_vector3() {
    for &(input, expected_x, expected_y, expected_z, default_x, default_y, default_z) in DICTIONARY_PARSE_VECTOR3 {
        let (_, tree) = parse(input);
        let default_value = Vector3::new(default_x, default_y, default_z);
        let actual_value = tree.root().parse_vector3("key", default_value).expect("no exception");
        assert_eq!(Vector3::new(expected_x, expected_y, expected_z), actual_value, "for {input:?}");
    }
}

#[test]
fn legacy_parse_vector3() {
    for &(input, expected_x, expected_y, expected_z) in LEGACY_PARSE_VECTOR3 {
        let (_, tree) = parse(input);
        let actual_value = tree.root().legacy_parse_vector3("key").expect("no exception");
        assert_eq!(Vector3::new(expected_x, expected_y, expected_z), actual_value, "for {input:?}");
    }
}

/// `IndexOf(')', openingParenthesisIndex + 2)` throws when that start index is past the end, which happens when
/// the value ends with `(`. A start index equal to the length is allowed.
#[test]
fn a_parenthesis_at_the_end_throws() {
    let expected = MonoException::argument_out_of_range_message("startIndex", sr::ARGUMENT_OUT_OF_RANGE_INDEX);
    for input in ["(", "1, 2(", "1, 2, 3("] {
        let tree = tree_with_value(input);
        assert_eq!(Err(expected.clone()), value(&tree).try_parse_vector2(), "Vector2 for {input:?}");
        assert_eq!(Err(expected.clone()), value(&tree).try_parse_vector3(), "Vector3 for {input:?}");
        assert_eq!(Err(expected.clone()), value(&tree).parse_vector3(Vector3::default()), "for {input:?}");
        assert_eq!(Err(expected.clone()), tree.root().try_parse_vector2("key"), "dictionary for {input:?}");
        assert_eq!(Err(expected.clone()), tree.root().legacy_parse_vector3("key"), "legacy for {input:?}");
    }
    let tree = tree_with_value("(1");
    assert_eq!(Ok(None), value(&tree).try_parse_vector2());
    assert_eq!(Ok(None), value(&tree).try_parse_vector3());
}

/// A dictionary under the key always succeeds, with 0 for a missing or unreadable axis.
#[test]
fn a_dictionary_always_succeeds() {
    let (_, tree) = parse("key\n{\nX 1\nY oops\n}");
    assert_eq!(Ok(Some(Vector2::new(1.0, 0.0))), tree.root().try_parse_vector2("key"));
    assert_eq!(Ok(Some(Vector3::new(1.0, 0.0, 0.0))), tree.root().try_parse_vector3("key"));
    let (_, tree) = parse("key\n[\n1\n]");
    assert_eq!(Ok(None), tree.root().try_parse_vector3("key"));
}
