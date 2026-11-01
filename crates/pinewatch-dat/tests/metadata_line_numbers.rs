//! Line numbers: every row of `Assets/Tests/UnturnedDat/DatParserMetadataLineNumberTests.cs`, one table per C# test
//! method, plus how the parser counts lines and what it records for unclosed brackets.

mod common;

use common::{
    assert_no_error, found, get_dictionary, get_list, get_value, list_get_value, parse, parse_with_metadata,
};
use pinewatch_dat::NodeRef;

/// `RootValueLineNumbers`.
const ROOT_VALUE_LINE_NUMBERS: &[(&str, &str, i32)] = &[
    ("key1 value1\nkey2 value2", "key1", 1),
    ("key1 value1\nkey2 value2", "key2", 2),
    ("key1 value1\nkey2\n[\n1\n]\nkey3 value3", "key3", 6),
    ("flag", "flag", 1),
    ("flag1\nflag2", "flag1", 1),
    ("flag1\nflag2", "flag2", 2),
    ("flag1\n\nflag2\n", "flag2", 3),
    ("flag1\n\n\nflag2\n", "flag2", 4),
];

/// `RootListLineNumbers`.
const ROOT_LIST_LINE_NUMBERS: &[(&str, &str, i32, i32)] = &[
    ("list\n[\n]", "list", 2, 3),
    ("list\n[\n1\n2\n]", "list", 2, 5),
    ("key1 value1\n\nlist\n[\n1\n2\n]", "list", 4, 7),
];

/// `RootDictionaryLineNumbers`.
const ROOT_DICTIONARY_LINE_NUMBERS: &[(&str, &str, i32, i32)] = &[
    ("dict\n{\n}", "dict", 2, 3),
    ("dict\n{\nkey1 value1\n}", "dict", 2, 4),
];

/// `DictionaryValueLineNumbers`.
const DICTIONARY_VALUE_LINE_NUMBERS: &[(&str, &str, i32)] = &[
    ("dict\n{\nkey1 value1\nkey2 value2\n}", "key1", 3),
    ("dict\n{\nkey1 value1\nkey2 value2\n}", "key2", 4),
];

/// `ListValueLineNumbers`.
const LIST_VALUE_LINE_NUMBERS: &[(&str, i32, i32)] = &[
    ("list\n[\nvalue1\nvalue2\n]", 0, 3),
    ("list\n[\nvalue1\nvalue2\n]", 1, 4),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("RootValueLineNumbers", ROOT_VALUE_LINE_NUMBERS.len()),
    ("RootListLineNumbers", ROOT_LIST_LINE_NUMBERS.len()),
    ("RootDictionaryLineNumbers", ROOT_DICTIONARY_LINE_NUMBERS.len()),
    ("DictionaryValueLineNumbers", DICTIONARY_VALUE_LINE_NUMBERS.len()),
    ("ListValueLineNumbers", LIST_VALUE_LINE_NUMBERS.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 17, "{TABLES:?}");
}

#[test]
fn root_value_line_numbers() {
    for &(input, key, expected_line_number) in ROOT_VALUE_LINE_NUMBERS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let value = found(get_value(tree.root(), key), "TryGetValue", input);
        assert_eq!(Some(expected_line_number), NodeRef::from(value).parsed_line_number(), "{input:?}");
    }
}

#[test]
fn root_list_line_numbers() {
    for &(input, key, expected_starting_line_number, expected_ending_line_number) in ROOT_LIST_LINE_NUMBERS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), key), "TryGetList", input);
        let range = NodeRef::from(list).parsed_line_number_range();
        assert_eq!(Some((expected_starting_line_number, expected_ending_line_number)), range, "{input:?}");
    }
}

#[test]
fn root_dictionary_line_numbers() {
    for &(input, key, expected_starting_line_number, expected_ending_line_number) in ROOT_DICTIONARY_LINE_NUMBERS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let dictionary = found(get_dictionary(tree.root(), key), "TryGetDictionary", input);
        let range = NodeRef::from(dictionary).parsed_line_number_range();
        assert_eq!(Some((expected_starting_line_number, expected_ending_line_number)), range, "{input:?}");
    }
}

#[test]
fn dictionary_value_line_numbers() {
    for &(input, key, expected_line_number) in DICTIONARY_VALUE_LINE_NUMBERS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let dictionary = found(get_dictionary(tree.root(), "dict"), "GetDictionary(dict)", input);
        let value = found(get_value(dictionary, key), "TryGetValue", input);
        assert_eq!(Some(expected_line_number), NodeRef::from(value).parsed_line_number(), "{input:?}");
    }
}

#[test]
fn list_value_line_numbers() {
    for &(input, index, expected_line_number) in LIST_VALUE_LINE_NUMBERS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "list"), "GetList(list)", input);
        let value = found(list_get_value(list, index), "TryGetValue", input);
        assert_eq!(Some(expected_line_number), NodeRef::from(value).parsed_line_number(), "{input:?}");
    }
}

/// The parser counts line-break tokens. The break inside a quoted string isn't one, so `b`, on physical line 3, is
/// parsed as line 2. The tokenizer's own messages count physical lines.
#[test]
fn line_numbers_drift_after_multiline_quoted_strings() {
    let (parser, tree) = parse_with_metadata("a \"x\ny\"\nb 1");
    assert_no_error(&parser, "drift");
    let b = found(get_value(tree.root(), "b"), "b", "drift");
    assert_eq!(Some(2), NodeRef::from(b).parsed_line_number());
    assert_eq!(Some((2, 2)), NodeRef::from(b).parsed_line_number_range());
}

/// The root starts on line 1 and ends on the parser's last line, which counts a trailing line break.
#[test]
fn root_dictionary_spans_the_file() {
    let (_, tree) = parse_with_metadata("a 1\nb 2\n");
    assert_eq!(Some(1), NodeRef::from(tree.root()).parsed_line_number());
    assert_eq!(Some((1, 3)), NodeRef::from(tree.root()).parsed_line_number_range());

    let (_, tree) = parse_with_metadata("a 1");
    assert_eq!(Some((1, 1)), NodeRef::from(tree.root()).parsed_line_number_range());
}

/// A dictionary or list records its lines only when its closing bracket is found, so an unclosed one reports 0.
#[test]
fn unclosed_brackets_report_line_zero() {
    let (_, tree) = parse_with_metadata("d\n{\nk v");
    let dictionary = found(get_dictionary(tree.root(), "d"), "d", "unclosed {");
    assert_eq!(Some(0), NodeRef::from(dictionary).parsed_line_number());
    assert_eq!(Some((0, 0)), NodeRef::from(dictionary).parsed_line_number_range());
    let k = found(get_value(dictionary, "k"), "k", "unclosed {");
    assert_eq!(Some(3), NodeRef::from(k).parsed_line_number());

    let (_, tree) = parse_with_metadata("l\n[\nx");
    let list = found(get_list(tree.root(), "l"), "l", "unclosed [");
    assert_eq!(Some((0, 0)), NodeRef::from(list).parsed_line_number_range());
    let x = found(list_get_value(list, 0), "x", "unclosed [");
    assert_eq!(Some(3), NodeRef::from(x).parsed_line_number());
}

/// `TryGetKeyLineNumber` passes through to the plain `DatDictionary`, which never has one.
#[test]
fn key_line_number_is_never_available() {
    let (_, tree) = parse_with_metadata("a 1\nd\n{\nk v\n}");
    assert_eq!(None, tree.root().key_line_number("a"));
    let dictionary = found(get_dictionary(tree.root(), "d"), "d", "key line");
    assert_eq!(None, dictionary.key_line_number("k"));
}

/// Without metadata, no node has line numbers.
#[test]
fn no_line_numbers_without_metadata() {
    let (_, tree) = parse("a 1\nl\n[\nx\n]");
    assert_eq!(None, NodeRef::from(tree.root()).parsed_line_number());
    let value = found(get_value(tree.root(), "a"), "a", "plain");
    assert_eq!(None, NodeRef::from(value).parsed_line_number());
    let list = found(get_list(tree.root(), "l"), "l", "plain");
    assert_eq!(None, NodeRef::from(list).parsed_line_number_range());
}
