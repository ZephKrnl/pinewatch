//! Parser errors: every row of `Assets/Tests/UnturnedDat/DatParserErrorTests.cs`, one table per C# test method,
//! plus the exact messages for keys that differ only by case and for input that ends early.

mod common;

use common::{assert_error, assert_string, found, get_dictionary, get_list, get_string, list_get_string, parse};
use pinewatch_mono::NetString;

/// `DictionaryWithoutClosingBrace`.
const DICTIONARY_WITHOUT_CLOSING_BRACE: &[&str] = &[
    "key1\n{",
    "key1\n{\n",
    "key1\n{\nkey2 value2",
    "key1\n{\nkey2 value2\n",
];

/// `ListWithoutClosingBracket`.
const LIST_WITHOUT_CLOSING_BRACKET: &[&str] = &[
    "key\n[",
    "key\n[\n",
    "key\n[\nvalue",
    "key\n[\nvalue\nvalue\n",
];

/// `StringWithoutClosingQuote`.
const STRING_WITHOUT_CLOSING_QUOTE: &[&str] = &[
    "\"",
    " \"",
    "\n\"",
    "\"\n",
    "\n\"\n",
    "key\n\"",
    "key\n\" ",
    "key\n\"\nvalue",
    "key\n\"\nvalue\nvalue\n",
];

/// `LineNumberCounting`.
const LINE_NUMBER_COUNTING: &[(&str, i32)] = &[
    ("\"", 1),
    ("\"\n", 1),
    ("\"\r", 1),
    ("\"\r\n", 1),
    ("\n\"", 2),
    ("\r\"", 2),
    ("\r\n\"", 2),
    ("\n\"\n", 2),
    ("\r\"\r", 2),
    ("\r\n\"\r\n", 2),
    ("\n\"\n\n", 2),
    ("\r\"\r\r", 2),
    ("\r\n\"\r\n\r\n", 2),
    ("\n\n\"\n", 3),
    ("\r\r\"\r", 3),
    ("\r\n\r\n\"\r\n", 3),
    ("\n\n\"\n\n", 3),
    ("\r\r\"\r\r", 3),
    ("\r\n\r\n\"\r\n\r\n", 3),
];

/// `DuplicateKey`.
const DUPLICATE_KEY: &[(&str, &str)] = &[
    ("key value1\nkey value2", "value2"),
];

/// `InvalidEscapeSequence`.
const INVALID_ESCAPE_SEQUENCE: &[(&str, &str)] = &[
    ("key path\\of\\asset", "path\\of\\asset"),
    ("key \"path\\of\\asset\"", "path\\of\\asset"),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("DictionaryWithoutClosingBrace", DICTIONARY_WITHOUT_CLOSING_BRACE.len()),
    ("ListWithoutClosingBracket", LIST_WITHOUT_CLOSING_BRACKET.len()),
    ("StringWithoutClosingQuote", STRING_WITHOUT_CLOSING_QUOTE.len()),
    ("LineNumberCounting", LINE_NUMBER_COUNTING.len()),
    ("DuplicateKey", DUPLICATE_KEY.len()),
    ("InvalidEscapeSequence", INVALID_ESCAPE_SEQUENCE.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 39, "{TABLES:?}");
}

/// The tables whose rows only check `parser.HasError`.
fn assert_errors(rows: &[&str]) {
    for &input in rows {
        let (parser, _) = parse(input);
        assert_error(&parser, input);
    }
}

#[test]
fn dictionary_without_closing_brace() {
    assert_errors(DICTIONARY_WITHOUT_CLOSING_BRACE);
}

#[test]
fn list_without_closing_bracket() {
    assert_errors(LIST_WITHOUT_CLOSING_BRACKET);
}

#[test]
fn string_without_closing_quote() {
    assert_errors(STRING_WITHOUT_CLOSING_QUOTE);
}

/// Error message should specify line number regardless of \n, \r, or \r\n.
#[test]
fn line_number_counting() {
    for &(input, expected_error_line_number) in LINE_NUMBER_COUNTING {
        let (parser, _) = parse(input);
        assert_error(&parser, input);
        let message = found(parser.error_message(), "ErrorMessage", input);
        let suffix: Vec<u16> = format!("string opened on line {expected_error_line_number}").encode_utf16().collect();
        assert!(message.as_units().ends_with(&suffix), "{message:?} for {input:?}");
    }
}

/// Duplicate keys should log an error and use the latest value.
#[test]
fn duplicate_key() {
    for &(input, expected_value) in DUPLICATE_KEY {
        let (parser, tree) = parse(input);
        assert_error(&parser, input);
        assert_string(expected_value, get_string(tree.root(), "key"), input);
    }
}

/// 2023-05-27: 3.23.7.0 added '\n' handling for unquoted strings, which broke mods using '\' in file paths. Most of
/// those still work: an unrecognized escape sequence is kept as '\' and reported.
#[test]
fn invalid_escape_sequence() {
    for &(input, expected_value) in INVALID_ESCAPE_SEQUENCE {
        let (parser, tree) = parse(input);
        assert_error(&parser, input);
        assert_string(expected_value, get_string(tree.root(), "key"), input);
    }
}

/// The pairs of a dictionary, with null values as `None`.
fn pairs(dictionary: pinewatch_dat::DictRef<'_>) -> Vec<(NetString, Option<NetString>)> {
    dictionary
        .iter()
        .map(|(key, node)| (key.clone(), node.as_value().and_then(|value| value.value()).cloned()))
        .collect()
}

/// Keys compare with `OrdinalIgnoreCase`, so `KEY` and `ÉTÉ` replace the values of `Key` and `été`, which keep their
/// spelling. Each message names the new spelling and the line the parser has reached: past the value's line break.
#[test]
fn duplicate_key_with_other_case_keeps_first_spelling() {
    let (parser, tree) = parse("Key 1\nKEY 2\n\u{00E9}t\u{00E9} a\n\u{00C9}T\u{00C9} b");
    assert_eq!(
        parser.error_messages(),
        [
            "duplicate key \"KEY\" on line 3 replacing existing value \"1\" with \"2\"",
            "duplicate key \"\u{00C9}T\u{00C9}\" on line 4 replacing existing value \"a\" with \"b\"",
        ]
    );
    let expected = [("Key", "2"), ("\u{00E9}t\u{00E9}", "b")]
        .map(|(key, value)| (NetString::from(key), Some(NetString::from(value))));
    assert_eq!(pairs(tree.root()), expected);
}

/// `OrdinalIgnoreCase` folds every code unit through Mono's invariant `TextInfo.ToUpper` table (V3), which leaves
/// `ſ` (U+017F) and `ı` (U+0131) as they are but maps `ᾀ` (U+1F80) to `ᾈ` (U+1F88). So `ſize` and `ıtem` are new
/// keys, while `ᾈ` replaces the value of `ᾀ`.
#[test]
fn key_case_folding_follows_monos_table() {
    let (parser, tree) = parse("Size 1\n\u{17F}ize 2\nItem 3\n\u{131}tem 4\n\u{1F80} 5\n\u{1F88} 6");
    assert_eq!(
        parser.error_messages(),
        ["duplicate key \"\u{1F88}\" on line 6 replacing existing value \"5\" with \"6\""]
    );
    let expected = [("Size", "1"), ("\u{17F}ize", "2"), ("Item", "3"), ("\u{131}tem", "4"), ("\u{1F80}", "6")]
        .map(|(key, value)| (NetString::from(key), Some(NetString::from(value))));
    assert_eq!(pairs(tree.root()), expected);
}

/// The message describes the replaced and the new node: a quoted value, `value(null)` for a flag, or a list or
/// dictionary with its item count.
#[test]
fn duplicate_key_messages_describe_both_nodes() {
    let (parser, _) = parse("a 1\na\na\n[\nx\ny\n]\na\n{\nk v\n}\na \"\"");
    assert_eq!(
        parser.error_messages(),
        [
            "duplicate key \"a\" on line 3 replacing existing value \"1\" with value(null)",
            "duplicate key \"a\" on line 8 replacing existing value value(null) with list with 2 item(s)",
            "duplicate key \"a\" on line 12 replacing existing value list with 2 item(s) with dictionary with 1 item(s)",
            "duplicate key \"a\" on line 12 replacing existing value dictionary with 1 item(s) with \"\"",
        ]
    );
}

/// Input that ends early: inside a quoted string, inside `{`, inside `[`, and right after a key. Parsing keeps what
/// it read, and the tokenizer's messages come before the parser's, innermost bracket first.
#[test]
fn truncated_input() {
    let (parser, tree) = parse("a 1\nb \"unterminated");
    assert_eq!(parser.error_messages(), ["missing closing quotation mark (\") for string opened on line 2"]);
    assert_string("1", get_string(tree.root(), "a"), "quote");
    assert_string("unterminated", get_string(tree.root(), "b"), "quote");

    let (parser, tree) = parse("a 1\nd\n{\nk v");
    assert_eq!(parser.error_messages(), ["missing closing curly bracket '}' for dictionary opened on line 3"]);
    assert_string("1", get_string(tree.root(), "a"), "dictionary");
    let dictionary = found(get_dictionary(tree.root(), "d"), "d", "dictionary");
    assert_string("v", get_string(dictionary, "k"), "dictionary");

    let (parser, tree) = parse("a 1\nl\n[\nx\ny");
    assert_eq!(parser.error_messages(), ["missing closing bracket ']' for list opened on line 3"]);
    let list = found(get_list(tree.root(), "l"), "l", "list");
    assert_eq!(2, list.len());
    assert_string("x", list_get_string(list, 0), "list");
    assert_string("y", list_get_string(list, 1), "list");

    let (parser, tree) = parse("a 1\nb");
    assert!(!parser.has_error(), "{:?}", parser.error_messages());
    let b = found(tree.root().try_get_node("b").and_then(|node| node.as_value()), "b", "key");
    assert_eq!(None, b.value());

    let (parser, tree) = parse("d\n{\nl\n[\n\"x");
    assert_eq!(
        parser.error_messages(),
        [
            "missing closing quotation mark (\") for string opened on line 5",
            "missing closing bracket ']' for list opened on line 4",
            "missing closing curly bracket '}' for dictionary opened on line 2",
        ]
    );
    let list = found(get_dictionary(tree.root(), "d").and_then(|d| get_list(d, "l")), "d/l", "nested");
    assert_string("x", list_get_string(list, 0), "nested");
}

/// A parser is reused between files: each parse starts with no messages and line 1.
#[test]
fn reused_parser_starts_fresh() {
    let mut parser = pinewatch_dat::DatParser::new();
    parser.parse_str("a\n{");
    assert!(parser.has_error());
    let tree = parser.parse_str("b 1\nb 2");
    assert_eq!(parser.error_messages(), ["duplicate key \"b\" on line 2 replacing existing value \"1\" with \"2\""]);
    assert_string("2", get_string(tree.root(), "b"), "second");
}
