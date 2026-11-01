//! Prefix comments: every row of `Assets/Tests/UnturnedDat/DatParserMetadataPrefixCommentTests.cs`, one table per
//! C# test method, plus a reused parser, empty comment lines and the dump.

mod common;

use common::{assert_no_error, found, get_dictionary, get_list, get_value, parse, parse_with_metadata};
use pinewatch_dat::{DatParser, NodeRef};
use pinewatch_mono::{NEW_LINE, NetString};

/// `RootValueComments`.
const ROOT_VALUE_COMMENTS: &[(&str, &str, Option<&str>, i32, i32)] = &[
    ("key value", "key", None, -1, -1),
    ("// A comment\nkey value", "key", Some("A comment"), 1, 1),
    ("// part 1 of 2\n// part 2 of 2\nkey value", "key", Some("part 1 of 2\npart 2 of 2"), 1, 2),
    ("key1 value1\n\nkey2 value2", "key1", None, -1, -1),
    ("key1 value1\n\nkey2 value2", "key2", None, -1, -1),
    ("key1 value1\n\n// A comment\nkey2 value2", "key1", None, -1, -1),
    ("key1 value1\n\n// A comment\nkey2 value2", "key2", Some("A comment"), 3, 3),
];

/// `RootDictionaryComments`.
const ROOT_DICTIONARY_COMMENTS: &[(&str, &str, Option<&str>, i32, i32)] = &[
    ("Dict1\n{\n}\nDict2\n{\n}", "Dict1", None, -1, -1),
    ("Dict1\n{\n}\nDict2\n{\n}", "Dict2", None, -1, -1),
    ("// Comment A\nDict1\n{\n}\n// Comment B\nDict2\n{\n}", "Dict1", Some("Comment A"), 1, 1),
    ("// Comment A\nDict1\n{\n}\n// Comment B\nDict2\n{\n}", "Dict2", Some("Comment B"), 5, 5),
    ("// Comment A1\n// Comment A2\nDict1\n{\n}\n// Comment B1\n//Comment B2\nDict2\n{\n}", "Dict1", Some("Comment A1\nComment A2"), 1, 2),
    ("// Comment A1\n// Comment A2\nDict1\n{\n}\n// Comment B1\n//Comment B2\nDict2\n{\n}", "Dict2", Some("Comment B1\nComment B2"), 6, 7),
];

/// `RootListComments`.
const ROOT_LIST_COMMENTS: &[(&str, &str, Option<&str>, i32, i32)] = &[
    ("List1\n[\n]\nList2\n[\n]", "List1", None, -1, -1),
    ("List1\n[\n]\nList2\n[\n]", "List2", None, -1, -1),
    ("// Comment A\nList1\n[\n]\n// Comment B\nList2\n[\n]", "List1", Some("Comment A"), 1, 1),
    ("// Comment A\nList1\n[\n]\n// Comment B\nList2\n[\n]", "List2", Some("Comment B"), 5, 5),
    ("// Comment A1\n// Comment A2\nList1\n[\n]\n// Comment B1\n//Comment B2\nList2\n[\n]", "List1", Some("Comment A1\nComment A2"), 1, 2),
    ("// Comment A1\n// Comment A2\nList1\n[\n]\n// Comment B1\n//Comment B2\nList2\n[\n]", "List2", Some("Comment B1\nComment B2"), 6, 7),
];

/// `DictionaryComments`.
const DICTIONARY_COMMENTS: &[(&str, &str, &str, i32, i32)] = &[
    ("Dict\n{\nFlag\n// Comment A\nKey1 Value1\n}", "Key1", "Comment A", 4, 4),
    ("Dict\n{\nFlag\n// Comment A\n// Comment B\nKey1 Value1\n}", "Key1", "Comment A\nComment B", 4, 5),
    ("Dict\n{\n// Comment A\n//Comment B\nKey1 // Inline Comment\n{\n}\n\n}", "Key1", "Comment A\nComment B", 3, 4),
    ("Dict\n{\n// Comment A\n//Comment B\nKey1 // Inline Comment\n[\n]\n\n}", "Key1", "Comment A\nComment B", 3, 4),
    ("Dict\n{\n// Comment A\n//Comment B\nKey1\n{\n}\n// Comment C\n//Comment D\nKey2\n{\n}\n}", "Key1", "Comment A\nComment B", 3, 4),
    ("Dict\n{\n// Comment A\n//Comment B\nKey1\n{\n}\n// Comment C\n//Comment D\nKey2\n{\n}\n}", "Key2", "Comment C\nComment D", 8, 9),
    ("Dict\n{\n// Comment A\n//Comment B\nKey1\n[\n]\n// Comment C\n//Comment D\nKey2\n[\n]\n}", "Key1", "Comment A\nComment B", 3, 4),
    ("Dict\n{\n// Comment A\n//Comment B\nKey1\n[\n]\n// Comment C\n//Comment D\nKey2\n[\n]\n}", "Key2", "Comment C\nComment D", 8, 9),
];

/// `ListComments`.
const LIST_COMMENTS: &[(&str, i32, Option<&str>, i32, i32)] = &[
    ("List\n[\nValue 1\n// Comment A\nValue2\n]", 0, None, -1, -1),
    ("List\n[\nValue 1\n// Comment A\nValue2\n]", 1, Some("Comment A"), 4, 4),
    ("List\n[\n// Comment A\nValue1\n// Comment B\nValue2\n]", 0, Some("Comment A"), 3, 3),
    ("List\n[\n// Comment A\nValue1\n// Comment B\nValue2\n]", 1, Some("Comment B"), 5, 5),
    ("List\n[\n// Comment A\n// Comment B\nValue1\n// Comment C\n// Comment D\nValue2\n]", 0, Some("Comment A\nComment B"), 3, 4),
    ("List\n[\n// Comment A\n// Comment B\nValue1\n// Comment C\n// Comment D\nValue2\n]", 1, Some("Comment C\nComment D"), 6, 7),
    ("List\n[\n// Comment A\n// Comment B\n[\n]\n// Comment C\n// Comment D\n[\n]\n]", 0, Some("Comment A\nComment B"), 3, 4),
    ("List\n[\n// Comment A\n// Comment B\n[\n]\n// Comment C\n// Comment D\n[\n]\n]", 1, Some("Comment C\nComment D"), 7, 8),
    ("List\n[\n// Comment A\n// Comment B\n{\n}\n// Comment C\n// Comment D\n{\n}\n]", 0, Some("Comment A\nComment B"), 3, 4),
    ("List\n[\n// Comment A\n// Comment B\n{\n}\n// Comment C\n// Comment D\n{\n}\n]", 1, Some("Comment C\nComment D"), 7, 8),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("RootValueComments", ROOT_VALUE_COMMENTS.len()),
    ("RootDictionaryComments", ROOT_DICTIONARY_COMMENTS.len()),
    ("RootListComments", ROOT_LIST_COMMENTS.len()),
    ("DictionaryComments", DICTIONARY_COMMENTS.len()),
    ("ListComments", LIST_COMMENTS.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 37, "{TABLES:?}");
}

/// The C# assertions on `TryGetParsedComment`: a comment exists exactly when one is expected, and then its
/// `MessageWithLineBreaks` and line numbers match.
#[track_caller]
fn assert_prefix_comment(
    node: NodeRef<'_>,
    expected_comment: Option<&str>,
    expected_starting_line_number: i32,
    expected_ending_line_number: i32,
    input: &str,
) {
    let comment = node.parsed_comment();
    let expect_comment = expected_comment.is_some_and(|comment| !comment.is_empty());
    assert_eq!(expect_comment, comment.is_some(), "hasComment for {input:?}");
    if let Some(comment) = comment {
        assert_eq!(expected_comment.map(NetString::from), comment.message_with_line_breaks(), "for {input:?}");
        assert_eq!(expected_starting_line_number, comment.starting_line_number, "starting line for {input:?}");
        assert_eq!(expected_ending_line_number, comment.ending_line_number, "ending line for {input:?}");
    }
}

#[test]
fn root_value_comments() {
    for &(input, key, expected_comment, starting_line_number, ending_line_number) in ROOT_VALUE_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let node = found(get_value(tree.root(), key), "TryGetValue", input);
        assert_prefix_comment(node.into(), expected_comment, starting_line_number, ending_line_number, input);
    }
}

/// Test that prefix comments are associated with root-level dictionaries properly.
#[test]
fn root_dictionary_comments() {
    for &(input, key, expected_comment, starting_line_number, ending_line_number) in ROOT_DICTIONARY_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let node = found(get_dictionary(tree.root(), key), "TryGetDictionary", input);
        assert_prefix_comment(node.into(), expected_comment, starting_line_number, ending_line_number, input);
    }
}

/// Test that prefix comments are associated with root-level lists properly.
#[test]
fn root_list_comments() {
    for &(input, key, expected_comment, starting_line_number, ending_line_number) in ROOT_LIST_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let node = found(get_list(tree.root(), key), "TryGetList", input);
        assert_prefix_comment(node.into(), expected_comment, starting_line_number, ending_line_number, input);
    }
}

/// Test that prefix comments are associated with items in dictionaries properly.
#[test]
fn dictionary_comments() {
    for &(input, key, expected_comment, starting_line_number, ending_line_number) in DICTIONARY_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let child_dictionary = found(get_dictionary(tree.root(), "Dict"), "TryGetDictionary", input);
        let node = found(child_dictionary.try_get_node(key), "TryGetNode", input);
        assert_prefix_comment(node, Some(expected_comment), starting_line_number, ending_line_number, input);
    }
}

/// Test that prefix comments are associated with items in lists properly.
#[test]
fn list_comments() {
    for &(input, index, expected_comment, starting_line_number, ending_line_number) in LIST_COMMENTS {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let child_list = found(get_list(tree.root(), "List"), "TryGetList", input);
        let node = found(child_list.try_get_node(index), "childList[index]", input);
        assert_prefix_comment(node, expected_comment, starting_line_number, ending_line_number, input);
    }
}

/// A parser is reused between files (spec section 6, P2). Comment lines left at the end of one file stay collected,
/// so they become the prefix comment of the next file's first node, with the first file's line numbers.
#[test]
fn reused_parser_carries_leftover_root_comment_into_next_file() {
    let mut parser = DatParser::new();
    parser.set_enable_metadata(true);
    parser.parse_str("a 1\n// trailing");
    let tree = parser.parse_str("key value");
    let key = NodeRef::from(found(get_value(tree.root(), "key"), "key", "second file"));
    let comment = found(key.parsed_comment(), "parsed comment", "second file");
    assert_eq!(Some(NetString::from("trailing")), comment.message_with_line_breaks());
    assert_eq!((2, 2), (comment.starting_line_number, comment.ending_line_number));

    let tree = parser.parse_str("key value");
    let key = NodeRef::from(found(get_value(tree.root(), "key"), "key", "third file"));
    assert!(key.parsed_comment().is_none());
}

/// An empty `//` line is a null message line: the comment still exists, a single null line joins to null, and
/// `string.Join` treats null lines as empty.
#[test]
fn empty_comment_lines_are_null() {
    let (_, tree) = parse_with_metadata("//\nkey value");
    let key = NodeRef::from(found(get_value(tree.root(), "key"), "key", "single"));
    let comment = found(key.parsed_comment(), "parsed comment", "single");
    assert_eq!(vec![None], comment.message_lines);
    assert_eq!(None, comment.message_with_line_breaks());

    let (_, tree) = parse_with_metadata("//\n// b\nkey value");
    let key = NodeRef::from(found(get_value(tree.root(), "key"), "key", "two"));
    let comment = found(key.parsed_comment(), "parsed comment", "two");
    assert_eq!(Some(NetString::from("\nb")), comment.message_with_line_breaks());
    assert_eq!(Some(NetString::from(" | b")), comment.join_lines(" | "));
}

/// Without metadata, comments are skipped and no node has one.
#[test]
fn no_comments_without_metadata() {
    let (_, tree) = parse("// A comment\nkey value");
    let key = NodeRef::from(found(get_value(tree.root(), "key"), "key", "plain"));
    assert!(key.parsed_comment().is_none());
}

/// With metadata, `DebugDumpToString` writes each child's prefix comment above it, one `// ` line per comment line
/// (`//` alone for an empty one). It still shows no line numbers or inline comments: a node with metadata dumps
/// through its underlying plain node.
#[test]
fn debug_dump_shows_prefix_comments() {
    let input = "// top\na \"1\" // inline\nd\n{\n// inner\n//\nk v\n}\nl\n[\n// first\nx\n]";
    let (parser, tree) = parse_with_metadata(input);
    assert_no_error(&parser, input);
    let expected: String = [
        "{",
        "\t// top",
        "\t\"a\" = \"1\"",
        "\t\"d\" = {",
        "\t\t// inner",
        "\t\t//",
        "\t\t\"k\" = \"v\"",
        "\t}",
        "\t\"l\" = [",
        "\t\t// first",
        "\t\t0 = \"x\"",
        "\t]",
        "}",
    ]
    .iter()
    .map(|line| format!("{line}{NEW_LINE}"))
    .collect();
    assert_eq!(NodeRef::from(tree.root()).debug_dump_to_string(), expected.as_str());
}
