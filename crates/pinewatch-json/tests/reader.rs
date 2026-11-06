//! `JsonTextReader.Read` with `JsonConvert`'s defaults, ported from Json.NET 13.0.3's `JsonTextReader.cs` and
//! `JsonReader.cs`. Every expected value here is also what the game's own `Newtonsoft.Json.dll` gives
//! (`reference/checks/jsontokens.cs`; the full corpus is `tests/corpus.rs`).

use pinewatch_json::{DateTimeKind, JsonDate, JsonInteger, JsonTextReader, JsonToken};
use pinewatch_mono::{ExceptionKind, MonoException, NetString};

fn reader(text: &str) -> JsonTextReader {
    JsonTextReader::new(NetString::from_str(text))
}

/// Every token `Read` returns, stopping at the first error.
fn tokens(text: &str) -> Result<Vec<JsonToken>, MonoException> {
    let mut reader = reader(text);
    let mut tokens = Vec::new();
    while reader.read()? {
        tokens.push(reader.token().clone());
    }
    Ok(tokens)
}

fn error(text: &str) -> String {
    let error = tokens(text).expect_err(text);
    assert_eq!(error.kind, ExceptionKind::JsonReader, "{text}");
    error.message
}

fn s(text: &str) -> NetString {
    NetString::from_str(text)
}

fn int(value: i64) -> JsonToken {
    JsonToken::Integer(JsonInteger::I64(value))
}

#[test]
fn values_and_structure() {
    assert_eq!(tokens(r#"{"a": [1, -2.5, "s", true, null], 'b': undefined}"#).unwrap(), [
        JsonToken::StartObject,
        JsonToken::PropertyName(s("a")),
        JsonToken::StartArray,
        int(1),
        JsonToken::Float(-2.5),
        JsonToken::String(s("s")),
        JsonToken::Boolean(true),
        JsonToken::Null,
        JsonToken::EndArray,
        JsonToken::PropertyName(s("b")),
        JsonToken::Undefined,
        JsonToken::EndObject,
    ]);
    // A bare comma is a value: `ParseValue` sets `Undefined` without moving past it.
    assert_eq!(tokens("[1,,2]").unwrap(), [
        JsonToken::StartArray,
        int(1),
        JsonToken::Undefined,
        int(2),
        JsonToken::EndArray,
    ]);
}

#[test]
fn lenient_syntax() {
    assert_eq!(tokens("// x").unwrap(), [JsonToken::Comment(s(" x"))]);
    // A block comment keeps the space before `*/` (the plan said `" y"`; the shipped reader gives `" y "`).
    assert_eq!(tokens("/* y */").unwrap(), [JsonToken::Comment(s(" y "))]);
    // The end of the text after a comment returns false without resetting the token, as the original does.
    let mut comment = reader("// x");
    assert!(comment.read().unwrap());
    assert!(!comment.read().unwrap());
    assert_eq!(comment.token(), &JsonToken::Comment(s(" x")));
    // After a value the reader is finished, and the end resets the token to `None`.
    let mut value = reader("1");
    assert!(value.read().unwrap());
    assert!(!value.read().unwrap());
    assert_eq!(value.token(), &JsonToken::None);
    assert_eq!(tokens("{a_1$: 1}").unwrap(), [
        JsonToken::StartObject,
        JsonToken::PropertyName(s("a_1$")),
        int(1),
        JsonToken::EndObject,
    ]);
    assert_eq!(tokens("[1,]").unwrap(), [JsonToken::StartArray, int(1), JsonToken::EndArray]);
    // Constructors carry their name.
    assert_eq!(tokens("new Date(1)").unwrap(), [
        JsonToken::StartConstructor(s("Date")),
        int(1),
        JsonToken::EndConstructor,
    ]);
}

#[test]
fn numbers() {
    assert_eq!(tokens("0x1F").unwrap(), [int(31)]);
    assert_eq!(tokens("017").unwrap(), [int(15)]);
    assert_eq!(tokens("1e3").unwrap(), [JsonToken::Float(1000.0)]);
    assert!(matches!(tokens("NaN").unwrap()[..], [JsonToken::Float(f)] if f.is_nan()));
    assert_eq!(tokens("Infinity").unwrap(), [JsonToken::Float(f64::INFINITY)]);
    assert_eq!(tokens("-Infinity").unwrap(), [JsonToken::Float(f64::NEG_INFINITY)]);
    assert_eq!(tokens("9223372036854775807").unwrap(), [int(i64::MAX)]);
    assert_eq!(tokens("9223372036854775808").unwrap(), [JsonToken::Integer(JsonInteger::Big(
        "9223372036854775808".to_string()
    ))]);
    // Leading zeros after a minus sign are a BigInteger's digits, as BigInteger.Parse reads them.
    assert_eq!(tokens("-000000000000000000001").unwrap(), [JsonToken::Integer(JsonInteger::Big("-1".to_string()))]);
    assert_eq!(error("08"), "Input string '08' is not a valid number. Path '', line 1, position 2.");
    assert_eq!(error(&"1".repeat(381)), format!(
        "JSON integer {} is too large to parse. Path '', line 1, position 381.",
        "1".repeat(381)
    ));
}

#[test]
fn strings() {
    assert_eq!(tokens(r#""A\n\/""#).unwrap(), [JsonToken::String(s("A\n/"))]);
    assert_eq!(error(r#""\q""#), r"Bad JSON escape sequence: \q. Path '', line 1, position 3.");
    assert_eq!(error(r#""abc"#), r#"Unterminated string. Expected delimiter: ". Path '', line 1, position 4."#);
    // A surrogate pair stays a pair; a lone half becomes U+FFFD.
    assert_eq!(tokens(r#""😀""#).unwrap(), [JsonToken::String(NetString::from_units(vec![0xD83D, 0xDE00]))]);
    assert_eq!(tokens(r#""\uDE00""#).unwrap(), [JsonToken::String(NetString::from_units(vec![0xFFFD]))]);
    // A lone surrogate in the text itself passes through.
    let mut lone = JsonTextReader::new(NetString::from_units(vec![0x22, 0xD800, 0x22]));
    assert!(lone.read().unwrap());
    assert_eq!(lone.token(), &JsonToken::String(NetString::from_units(vec![0xD800])));
}

#[test]
fn dates() {
    let date = |ticks, kind| JsonToken::Date(JsonDate { ticks, kind });
    assert_eq!(tokens(r#""2012-03-21T05:40:00Z""#).unwrap(), [date(634_679_052_000_000_000, DateTimeKind::Utc)]);
    assert_eq!(tokens(r#""\/Date(1335205592410)\/""#).unwrap(), [date(634_708_023_924_100_000, DateTimeKind::Utc)]);
    assert_eq!(tokens(r#""2012-03-21T05:40:00""#).unwrap(), [date(
        634_679_052_000_000_000,
        DateTimeKind::Unspecified
    )]);
    // Json.NET only tries text of 19 to 40 characters with a `T` at index 10.
    assert_eq!(tokens(r#""2012-03-21""#).unwrap(), [JsonToken::String(s("2012-03-21"))]);
    // An offset converts to the local time zone, which is taken to be UTC until time zones are ported
    // (docs/VERIFICATION.md, V31): the token holds the UTC instant, marked `Local`.
    assert_eq!(tokens(r#""2012-03-21T05:40:00+01:00""#).unwrap(), [date(
        634_679_016_000_000_000,
        DateTimeKind::Local
    )]);
    // Adding the day for 24:00 on 9999-12-31 throws out of `DateTime.AddDays`.
    let error = tokens(r#""9999-12-31T24:00:00""#).unwrap_err();
    assert_eq!(error.kind, ExceptionKind::ArgumentOutOfRange);
    assert_eq!(error.message, format!(
        "The added or subtracted value results in an un-representable DateTime.{}Parameter name: value",
        pinewatch_mono::NEW_LINE
    ));
}

#[test]
fn depth_limit() {
    let deepest = format!("{}{}", "[".repeat(64), "]".repeat(64));
    assert_eq!(tokens(&deepest).unwrap().len(), 128);
    let too_deep = format!("{}{}", "[".repeat(65), "]".repeat(65));
    assert_eq!(error(&too_deep), format!(
        "The reader's MaxDepth of 64 has been exceeded. Path '{}', line 1, position 65.",
        "[0]".repeat(64)
    ));
}

#[test]
fn errors_name_the_position() {
    assert_eq!(
        error(r#"{"a": x}"#),
        "Unexpected character encountered while parsing value: x. Path 'a', line 1, position 6."
    );
    // Special characters put the name in brackets, escaped for single quotes.
    assert_eq!(
        error(r#"{"a'b": x}"#),
        r"Unexpected character encountered while parsing value: x. Path '['a\'b']', line 1, position 8."
    );
    assert_eq!(
        error("[1}"),
        "JsonToken EndObject is not valid for closing JsonType Array. Path '', line 1, position 3."
    );
}

#[test]
fn unicode_classification() {
    // `char.IsNumber` characters reach the number parser and fail there. Before the first token the line number is 0.
    assert_eq!(
        error("\u{b2}"),
        "Unexpected character encountered while parsing number: \u{b2}. Path '', line 0, position 0."
    );
    // Unquoted names take `char.IsLetterOrDigit` characters, Arabic-Indic digits included.
    assert_eq!(tokens("{\u{e4}\u{663}: 1}").unwrap()[1], JsonToken::PropertyName(s("\u{e4}\u{663}")));
    // `char.IsWhiteSpace` separates values; U+FEFF does not.
    assert_eq!(tokens("[1,\u{a0} 2]").unwrap(), [JsonToken::StartArray, int(1), int(2), JsonToken::EndArray]);
    assert_eq!(
        error("\u{feff}[1]"),
        "Unexpected character encountered while parsing value: \u{feff}. Path '', line 0, position 0."
    );
}

#[test]
fn line_and_position() {
    let mut reader = reader("[\n1,\r\n2,\r3]");
    let mut seen = Vec::new();
    while reader.read().unwrap() {
        seen.push((reader.line_number(), reader.line_position(), reader.depth(), reader.path()));
    }
    assert_eq!(seen, [
        (1, 1, 0, String::new()),
        (2, 1, 1, "[0]".to_string()),
        (3, 1, 1, "[1]".to_string()),
        (4, 1, 1, "[2]".to_string()),
        (4, 2, 0, String::new()),
    ]);
}
