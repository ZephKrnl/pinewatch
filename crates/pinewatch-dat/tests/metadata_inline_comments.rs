//! Inline comments: every row of `Assets/Tests/UnturnedDat/DatParserMetadataInlineCommentTests.cs`, one table per
//! C# test method. An inline comment follows a quoted value. After an unquoted value, `//` is part of the value.

mod common;

use common::{assert_no_error, found, get_dictionary, get_list, get_value, list_get_value, parse_with_metadata};
use pinewatch_mono::NetString;

/// `RootValueInlineComments`.
const ROOT_VALUE_INLINE_COMMENTS: &[(&str, &str, Option<&str>)] = &[
    ("key1 value1\nkey2 \"value2\"\nkey3 value3", "key1", None),
    ("key1 value1\nkey2 \"value2\"\nkey3 value3", "key2", None),
    ("key1 value1\nkey2 \"value2\"\nkey3 value3", "key3", None),
    ("key1 value1\nkey2 \"value2\" // A comment\nkey3 value3", "key1", None),
    ("key1 value1\nkey2 \"value2\" // A comment\nkey3 value3", "key2", Some("A comment")),
    ("key1 value1\nkey2 \"value2\" // A comment\nkey3 value3", "key3", None),
];

/// `DictionaryValueInlineComments`.
const DICTIONARY_VALUE_INLINE_COMMENTS: &[(&str, &str, Option<&str>)] = &[
    ("dict\n{\nkey1 value1\nkey2 \"value2\"\nkey3 value3\n}", "key1", None),
    ("dict\n{\nkey1 value1\nkey2 \"value2\"\nkey3 value3\n}", "key2", None),
    ("dict\n{\nkey1 value1\nkey2 \"value2\"\nkey3 value3\n}", "key3", None),
    ("dict\n{\nkey1 value1\nkey2 \"value2\" // A comment\nkey3 value3\n}", "key1", None),
    ("dict\n{\nkey1 value1\nkey2 \"value2\" // A comment\nkey3 value3\n}", "key2", Some("A comment")),
    ("dict\n{\nkey1 value1\nkey2 \"value2\" // A comment\nkey3 value3\n}", "key3", None),
];

/// `ListValueInlineComments`.
const LIST_VALUE_INLINE_COMMENTS: &[(&str, i32, Option<&str>)] = &[
    ("list\n[\nvalue1\n\"value2\"\nvalue3\n]", 0, None),
    ("list\n[\nvalue1\n\"value2\"\nvalue3\n]", 1, None),
    ("list\n[\nvalue1\n\"value2\"\nvalue3\n]", 2, None),
    ("list\n[\nvalue1\n\"value2\" // A comment\nvalue3\n]", 0, None),
    ("list\n[\nvalue1\n\"value2\" // A comment\nvalue3\n]", 1, Some("A comment")),
    ("list\n[\nvalue1\n\"value2\" // A comment\nvalue3\n]", 2, None),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("RootValueInlineComments", ROOT_VALUE_INLINE_COMMENTS.len()),
    ("DictionaryValueInlineComments", DICTIONARY_VALUE_INLINE_COMMENTS.len()),
    ("ListValueInlineComments", LIST_VALUE_INLINE_COMMENTS.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 18, "{TABLES:?}");
}

/// `Assert.AreEqual(expectedComment, actualComment)`.
#[track_caller]
fn assert_comment(expected_comment: Option<&str>, actual_comment: Option<&NetString>, input: &str) {
    assert_eq!(actual_comment, expected_comment.map(NetString::from).as_ref(), "for {input:?}");
}

#[test]
fn root_value_inline_comments() {
    for &(input, key, expected_comment) in ROOT_VALUE_INLINE_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let value = found(get_value(tree.root(), key), "TryGetValue", input);
        assert_comment(expected_comment, value.parsed_inline_comment(), input);
    }
}

#[test]
fn dictionary_value_inline_comments() {
    for &(input, key, expected_comment) in DICTIONARY_VALUE_INLINE_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let dictionary = found(get_dictionary(tree.root(), "dict"), "GetDictionary(dict)", input);
        let value = found(get_value(dictionary, key), "TryGetValue", input);
        assert_comment(expected_comment, value.parsed_inline_comment(), input);
    }
}

#[test]
fn list_value_inline_comments() {
    for &(input, index, expected_comment) in LIST_VALUE_INLINE_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "list"), "GetList(list)", input);
        let value = found(list_get_value(list, index), "TryGetValue", input);
        assert_comment(expected_comment, value.parsed_inline_comment(), input);
    }
}

/// `//` with no text after a quoted value is an inline comment whose text is null.
#[test]
fn empty_inline_comment_is_null() {
    let (parser, tree) = parse_with_metadata("key \"value\" //\nnext \"value\"");
    assert_no_error(&parser, "empty");
    let value = found(get_value(tree.root(), "key"), "key", "empty");
    assert_eq!(None, value.parsed_inline_comment());
    assert_eq!(Some(&NetString::from("value")), value.value());
}
