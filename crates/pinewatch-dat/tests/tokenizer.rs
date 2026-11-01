//! The tokenizer: every case of `Assets/Tests/UnturnedDat/DatTokenizerTests.cs`, plus byte order marks, line
//! breaks, error messages and the token dump.

use pinewatch_dat::{DatToken, DatTokenType as T, DatTokenizer};
use pinewatch_mono::{NEW_LINE, NetString};

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// `new DatToken(type, value)`.
fn token(token_type: T, value: &str) -> DatToken {
    DatToken { token_type, value: Some(NetString::from(value)) }
}

/// `new DatToken(type)`.
fn bare(token_type: T) -> DatToken {
    DatToken { token_type, value: None }
}

fn tokenize(input: &[u16], enable_comments: bool) -> DatTokenizer {
    let mut tokenizer = DatTokenizer::new();
    tokenizer.set_enable_comments(enable_comments);
    tokenizer.tokenize(input);
    tokenizer
}

/// Reads every token, checking that each one is there and that nothing follows the last.
fn assert_tokens(tokenizer: &mut DatTokenizer, expected_tokens: &[DatToken]) {
    for (token_index, expected_token) in expected_tokens.iter().enumerate() {
        let actual_token = tokenizer.read_token().unwrap_or_else(|| panic!("tokens[{token_index}] missing"));
        assert_eq!(expected_token.token_type, actual_token.token_type, "tokens[{token_index}] type");
        assert_eq!(expected_token.value, actual_token.value, "tokens[{token_index}] value");
    }
    assert!(tokenizer.read_token().is_none(), "no more tokens");
}

/// `TestTokenizer`.
fn test_tokenizer(input: &str, expected_tokens: &[DatToken], enable_comments: bool) {
    let mut tokenizer = tokenize(&units(input), enable_comments);
    assert!(!tokenizer.has_error(), "{:?}", tokenizer.error_messages());
    assert_tokens(&mut tokenizer, expected_tokens);
}

#[test]
fn tokenize_flag() {
    test_tokenizer("Flag", &[token(T::Key, "Flag")], true);
}

/// Nelson 2025-04-16: debugging an issue where an extra line is inserted between flags.
#[test]
fn tokenize_flag_spacing() {
    let expected_tokens = [
        token(T::Key, "Flag1"),
        bare(T::LineBreak),
        bare(T::LineBreak),
        token(T::Key, "Flag2"),
        bare(T::LineBreak),
        token(T::Key, "Key"),
        token(T::Value, "Value"),
        bare(T::LineBreak),
        bare(T::LineBreak),
        token(T::Key, "Flag3"),
    ];
    test_tokenizer("Flag1\n\nFlag2\nKey Value\n\nFlag3", &expected_tokens, true);
}

#[test]
fn tokenize_key_value() {
    test_tokenizer("Key Value", &[token(T::Key, "Key"), token(T::Value, "Value")], true);
}

#[test]
fn tokenize_two_key_value_lines() {
    let expected_tokens = [
        token(T::Key, "Key1"),
        token(T::Value, "Value1"),
        bare(T::LineBreak),
        token(T::Key, "Key2"),
        token(T::Value, "Value2"),
    ];
    test_tokenizer("Key1 Value1\nKey2 Value2", &expected_tokens, true);
}

#[test]
fn tokenize_commented_key_value() {
    let expected_tokens = [
        token(T::Comment, "Comment A"),
        bare(T::LineBreak),
        token(T::Comment, "Comment B"),
        bare(T::LineBreak),
        token(T::Key, "Key"),
        token(T::Value, "Value"),
        token(T::Comment, "Comment C"),
    ];
    test_tokenizer("// Comment A\n//Comment B\nKey \"Value\" // Comment C", &expected_tokens, true);
}

#[test]
fn tokenize_dictionary() {
    let expected_tokens = [
        token(T::Key, "Dictionary"),
        bare(T::LineBreak),
        bare(T::OpenDictionary),
        bare(T::LineBreak),
        token(T::Key, "Key"),
        token(T::Value, "Value"),
        bare(T::LineBreak),
        bare(T::CloseDictionary),
    ];
    test_tokenizer("Dictionary\n{\nKey Value\n}", &expected_tokens, true);
}

#[test]
fn tokenize_list() {
    let expected_tokens = [
        token(T::Key, "List"),
        bare(T::LineBreak),
        bare(T::OpenList),
        bare(T::LineBreak),
        token(T::Comment, "Comment A"),
        bare(T::LineBreak),
        token(T::Value, "Value A"),
        bare(T::LineBreak),
        token(T::Value, "Value B"),
        token(T::Comment, "Comment B"),
        bare(T::LineBreak),
        bare(T::CloseList),
    ];
    test_tokenizer("List\n[\n// Comment A\nValue A\n\"Value B\" // Comment B\n]", &expected_tokens, true);
}

/// Test that enabling/disabling comment tokenization doesn't affect non-comment tokens. The C# input is a verbatim
/// string in a file with CRLF line ends and tab indents.
#[test]
fn compare_with_and_without_comments() {
    let input = concat!(
        "// A comment\r\n",
        "// Another comment\r\n",
        "Key1 \"Value1\" // Inline comment\r\n",
        "\r\n",
        "// Comment 4\r\n",
        "Key2 Value2\r\n",
        "List\r\n",
        "[\r\n",
        "\t// Comment 5\r\n",
        "\tList Item 1\r\n",
        "\r\n",
        "\t// Comment 6\r\n",
        "\t\"List Item 2\" // Another inline comment\r\n",
        "]\r\n",
        "\r\n",
        "// Comment 7\r\n",
        "// Comment 8\r\n",
        "Dictionary\r\n",
        "{\r\n",
        "\t// Comment 9\r\n",
        "\tKey3 Value3\r\n",
        "\tKey4 Value4\r\n",
        "}\r\n",
        "\r\n",
        "Key5 Mis-configured comment example // Not a comment\r\n",
        "Flag // With mis-configured comment",
    );

    let expected_tokens = [
        token(T::Comment, "A comment"),
        bare(T::LineBreak),
        token(T::Comment, "Another comment"),
        bare(T::LineBreak),
        token(T::Key, "Key1"),
        token(T::Value, "Value1"),
        token(T::Comment, "Inline comment"),
        bare(T::LineBreak),
        bare(T::LineBreak),
        token(T::Comment, "Comment 4"),
        bare(T::LineBreak),
        token(T::Key, "Key2"),
        token(T::Value, "Value2"),
        bare(T::LineBreak),
        token(T::Key, "List"),
        bare(T::LineBreak),
        bare(T::OpenList),
        bare(T::LineBreak),
        token(T::Comment, "Comment 5"),
        bare(T::LineBreak),
        token(T::Value, "List Item 1"),
        bare(T::LineBreak),
        bare(T::LineBreak),
        token(T::Comment, "Comment 6"),
        bare(T::LineBreak),
        token(T::Value, "List Item 2"),
        token(T::Comment, "Another inline comment"),
        bare(T::LineBreak),
        bare(T::CloseList),
        bare(T::LineBreak),
        bare(T::LineBreak),
        token(T::Comment, "Comment 7"),
        bare(T::LineBreak),
        token(T::Comment, "Comment 8"),
        bare(T::LineBreak),
        token(T::Key, "Dictionary"),
        bare(T::LineBreak),
        bare(T::OpenDictionary),
        bare(T::LineBreak),
        token(T::Comment, "Comment 9"),
        bare(T::LineBreak),
        token(T::Key, "Key3"),
        token(T::Value, "Value3"),
        bare(T::LineBreak),
        token(T::Key, "Key4"),
        token(T::Value, "Value4"),
        bare(T::LineBreak),
        bare(T::CloseDictionary),
        bare(T::LineBreak),
        bare(T::LineBreak),
        token(T::Key, "Key5"),
        token(T::Value, "Mis-configured comment example // Not a comment"),
        bare(T::LineBreak),
        token(T::Key, "Flag"),
        token(T::Value, "// With mis-configured comment"),
    ];

    test_tokenizer(input, &expected_tokens, true);

    let without_comments: Vec<DatToken> =
        expected_tokens.iter().filter(|token| token.token_type != T::Comment).cloned().collect();
    test_tokenizer(input, &without_comments, false);
}

/// `SkipUtf8Bom` compares characters, not bytes, and reads each one before comparing the next, so any prefix of
/// `EF BB BF` is swallowed. A character that doesn't start that sequence is kept.
#[test]
fn utf8_bom_characters_are_skipped_one_by_one() {
    let expected_tokens = [token(T::Key, "key"), token(T::Value, "value")];
    for prefix in [&[0xEF, 0xBB, 0xBF][..], &[0xEF], &[0xEF, 0xBB]] {
        let mut input = prefix.to_vec();
        input.extend(units("key value"));
        let mut tokenizer = tokenize(&input, true);
        assert!(!tokenizer.has_error());
        assert_tokens(&mut tokenizer, &expected_tokens);
    }

    let mut input = vec![0xBB];
    input.extend(units("key value"));
    assert_tokens(&mut tokenize(&input, true), &[token(T::Key, "\u{BB}key"), token(T::Value, "value")]);
}

#[test]
fn cr_lf_and_crlf_each_count_one_line() {
    let mut tokenizer = tokenize(&units("a 1\rb 2\nc 3\r\nd 4\\z"), true);
    assert_eq!(
        tokenizer.error_messages(),
        ["unrecognized escape sequence (\\z) on line 4 \u{2014} if this is a file path please use forward slash (/)"]
    );
    let expected_tokens = [
        token(T::Key, "a"),
        token(T::Value, "1"),
        bare(T::LineBreak),
        token(T::Key, "b"),
        token(T::Value, "2"),
        bare(T::LineBreak),
        token(T::Key, "c"),
        token(T::Value, "3"),
        bare(T::LineBreak),
        token(T::Key, "d"),
        token(T::Value, "4\\z"),
    ];
    assert_tokens(&mut tokenizer, &expected_tokens);
}

/// `\n` then `\r` is two line breaks.
#[test]
fn lf_then_cr_counts_two_lines() {
    let mut tokenizer = tokenize(&units("a\n\rb \"c"), true);
    assert_eq!(tokenizer.error_messages(), ["missing closing quotation mark (\") for string opened on line 3"]);
    assert_tokens(
        &mut tokenizer,
        &[token(T::Key, "a"), bare(T::LineBreak), bare(T::LineBreak), token(T::Key, "b"), token(T::Value, "c")],
    );
}

#[test]
fn unexpected_closing_brackets_are_reported_on_their_line() {
    let tokenizer = tokenize(&units("a\n}\n[\n}\n]\n]"), true);
    assert_eq!(
        tokenizer.error_messages(),
        [
            "unexpected end of dictionary/object '}' on line 2",
            "unexpected end of dictionary/object '}' on line 4",
            "unexpected end of list ']' on line 6",
        ]
    );
}

/// The escaped character goes into the message as it is, even a lone surrogate.
#[test]
fn escape_message_keeps_the_escaped_unit() {
    let mut input = units("key \"\\");
    input.push(0xD800);
    input.extend(units("\""));
    let tokenizer = tokenize(&input, true);
    let mut message = units("unrecognized escape sequence (\\");
    message.push(0xD800);
    message.extend(units(") on line 1 \u{2014} if this is a file path please use forward slash (/)"));
    assert_eq!(tokenizer.error_messages(), [NetString::from_units(message)]);
}

#[test]
fn reusing_a_tokenizer_starts_fresh() {
    let mut tokenizer = tokenize(&units("a\n\"b"), true);
    assert!(tokenizer.has_error());
    assert!(tokenizer.read_token().is_some());

    tokenizer.tokenize(&units("c \"d"));
    assert_eq!(tokenizer.error_messages(), ["missing closing quotation mark (\") for string opened on line 1"]);
    assert_tokens(&mut tokenizer, &[token(T::Key, "c"), token(T::Value, "d")]);
}

/// `DebugDumpTokensToString`: the index, the type's name, and the value unless it's null or empty, one line each.
#[test]
fn debug_dump_lists_each_token() {
    let tokenizer = tokenize(&units("Key Value\n//\nEmpty \"\""), true);
    let expected = [
        "0 Key Key",
        "1 Value Value",
        "2 LineBreak",
        "3 Comment",
        "4 LineBreak",
        "5 Key Empty",
        "6 Value",
    ]
    .map(|line| format!("{line}{NEW_LINE}"))
    .concat();
    assert_eq!(tokenizer.debug_dump_tokens_to_string(), expected.as_str());
}
