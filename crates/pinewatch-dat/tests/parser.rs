//! The parser: every row of `Assets/Tests/UnturnedDat/DatParserTests.cs`, one table per C# test method, plus
//! enumeration order, list lookups, node identity and `DebugDumpToString`.

mod common;

use common::{
    assert_no_error, assert_string, found, get_dictionary, get_list, get_string, list_get_dictionary,
    list_get_string, parse,
};
use pinewatch_dat::{DatNodeType, DatParser, NodeRef};
use pinewatch_mono::{Guid, NEW_LINE, NetString};

/// `IgnoreRootComments`.
const IGNORE_ROOT_COMMENTS: &[&str] = &[
    "// comment",
    "//comment without space",
    "// multiple\n// comments",
    " //",
    "//",
    "// ",
];

/// `IgnoreRootCommentsBeforeKey`.
const IGNORE_ROOT_COMMENTS_BEFORE_KEY: &[&str] = &[
    "// comment\nkey value",
    "//comment without space\nkey value",
    "// multiple\n// comments\nkey value",
    "//\nkey value",
];

/// `IgnoreDictionaryComments`.
const IGNORE_DICTIONARY_COMMENTS: &[&str] = &[
    "key\n{\n// comment\n}",
    "key\n{\n//comment without space\n}",
    "key\n{\n// multiple\n// comments\n}",
    "key\n{\n//\n}",
];

/// `IgnoreDictionaryCommentsBeforeKey`.
const IGNORE_DICTIONARY_COMMENTS_BEFORE_KEY: &[&str] = &[
    "key\n{\n// comment\nkey value\n}",
    "key\n{\n//comment without space\nkey value\n}",
    "key\n{\n// multiple\n// comments\nkey value\n}",
    "key\n{\n//\nkey value\n}",
];

/// `IgnoreDictionaryCommentsAfterKey`.
const IGNORE_DICTIONARY_COMMENTS_AFTER_KEY: &[&str] = &[
    "key\n{\nkey value\n// comment\n}",
    "key\n{\nkey value\n//comment without space\n}",
    "key\n{\nkey value\n// multiple\n// comments\n}",
    "key\n{\nkey value\n//\n}",
];

/// `IgnoreCommentsOnSameLineAsDictionaryValue`.
const IGNORE_COMMENTS_ON_SAME_LINE_AS_DICTIONARY_VALUE: &[(&str, &str, &str)] = &[
    ("key value //comment", "key", "value //comment"),
    ("key value // comment", "key", "value // comment"),
    ("key \"value\"//comment", "key", "value"),
    ("key \"value\"// comment", "key", "value"),
    ("key \"value\" //comment", "key", "value"),
    ("key \"value\" // comment", "key", "value"),
];

/// `IgnoreListComments`.
const IGNORE_LIST_COMMENTS: &[&str] = &[
    "key\n[\n// comment\n]",
    "key\n[\n//comment without space\n]",
    "key\n[\n// multiple\n// comments\n]",
    "key\n[\n//\n]",
];

/// `IgnoreListCommentsBeforeValue`.
const IGNORE_LIST_COMMENTS_BEFORE_VALUE: &[&str] = &[
    "key\n[\n// comment\nvalue\n]",
    "key\n[\n//comment without space\nvalue\n]",
    "key\n[\n// multiple\n// comments\nvalue\n]",
    "key\n[\n//\nvalue\n]",
];

/// `IgnoreListCommentsAfterValue`.
const IGNORE_LIST_COMMENTS_AFTER_VALUE: &[&str] = &[
    "key\n[\nvalue\n// comment\n]",
    "key\n[\nvalue\n//comment without space\n]",
    "key\n[\nvalue\n// multiple\n// comments\n]",
    "key\n[\nvalue\n//\n]",
];

/// `IgnoreCommentsOnSameLineAsListValue`.
const IGNORE_COMMENTS_ON_SAME_LINE_AS_LIST_VALUE: &[(&str, &str)] = &[
    ("key\n[\nvalue // comment\n]", "value // comment"),
    ("key\n[\nvalue //comment\n]", "value //comment"),
    ("key\n[\n\"value\" // comment\n]", "value"),
    ("key\n[\n\"value\" //comment\n]", "value"),
    ("key\n[\n\"value\"// comment\n]", "value"),
    ("key\n[\n\"value\"//comment\n]", "value"),
];

/// `ParseEmptyRootDictionary`.
const PARSE_EMPTY_ROOT_DICTIONARY: &[&str] = &[
    "",
    " ",
    "  ",
    "\n",
    "\n\n",
    " \n ",
    " \n \n",
    " \n \n ",
    "  \n  \n",
    "  \n  \n ",
    "  \n  \n  ",
];

/// `ParseValueBackslash`.
const PARSE_VALUE_BACKSLASH: &[(&str, &str, &str)] = &[
    ("key backslash: \\\\", "key", "backslash: \\"),
    ("key \"backslash: \\\\\"", "key", "backslash: \\"),
];

/// `ParseMultiLineString`.
const PARSE_MULTI_LINE_STRING: &[(&str, &str, &str)] = &[
    ("key \"\nvalue\"", "key", "\nvalue"),
    ("key \"value\n\"", "key", "value\n"),
    ("key \"\nvalue\n\"", "key", "\nvalue\n"),
    ("key \"line1\nline2\"", "key", "line1\nline2"),
    ("key \"line1\n\nline2\"", "key", "line1\n\nline2"),
];

/// `ParseFlag`.
const PARSE_FLAG: &[(&str, &str)] = &[
    ("flag", "flag"),
    (" flag", "flag"),
    ("flag ", "flag"),
    (" flag ", "flag"),
    ("flag\n", "flag"),
    (" flag\n", "flag"),
    (" flag \n", "flag"),
    (" flag \n ", "flag"),
];

/// `ParseMultipleFlags`.
const PARSE_MULTIPLE_FLAGS: &[(&str, &str, &str)] = &[
    ("flag1\nflag2", "flag1", "flag2"),
    ("flag1 \nflag2", "flag1", "flag2"),
    ("flag1\n flag2", "flag1", "flag2"),
    ("flag1 \n flag2", "flag1", "flag2"),
    (" flag1 \n flag2 ", "flag1", "flag2"),
];

/// `ParseValueWithTrailingSpaces`.
const PARSE_VALUE_WITH_TRAILING_SPACES: &[(&str, &str, &str)] = &[
    ("key value ", "key", "value "),
    ("key value \n", "key", "value "),
    ("key value \n ", "key", "value "),
    ("key value  ", "key", "value  "),
    ("key value  \n", "key", "value  "),
    ("key value  \n ", "key", "value  "),
    ("key \"value\" ", "key", "value"),
    ("key \"value\" \n", "key", "value"),
    ("key \"value\" \n ", "key", "value"),
    ("key \"value\"  \n ", "key", "value"),
];

/// `ParseSingleLineWithoutQuotes`.
const PARSE_SINGLE_LINE_WITHOUT_QUOTES: &[(&str, &str, &str)] = &[
    ("key value", "key", "value"),
    (" key value", "key", "value"),
    ("key  value", "key", "value"),
    (" key  value", "key", "value"),
    ("key value with space", "key", "value with space"),
    (" key value with space", "key", "value with space"),
    ("key  value with space", "key", "value with space"),
    (" key  value with space", "key", "value with space"),
];

/// `ParseMultiLineWithoutQuotes`.
const PARSE_MULTI_LINE_WITHOUT_QUOTES: &[(&str, &str, &str, &str, &str)] = &[
    ("key1 value1\nkey2 value2", "key1", "value1", "key2", "value2"),
    ("key1  value1\nkey2  value2", "key1", "value1", "key2", "value2"),
    (" key1 value1\n key2 value2", "key1", "value1", "key2", "value2"),
    ("key1 value with spaces 1\nkey2 value with spaces 2", "key1", "value with spaces 1", "key2", "value with spaces 2"),
    (" key1 value with spaces 1\n key2 value with spaces 2", "key1", "value with spaces 1", "key2", "value with spaces 2"),
    (" key1  value with spaces 1\n key2  value with spaces 2", "key1", "value with spaces 1", "key2", "value with spaces 2"),
];

/// `ParseQuotedKey`.
const PARSE_QUOTED_KEY: &[(&str, &str, &str)] = &[
    ("\"key\" value", "key", "value"),
    (" \"key\" value", "key", "value"),
    ("\"key\"  value", "key", "value"),
    (" \"key\"  value", "key", "value"),
    ("\"key with space\" value", "key with space", "value"),
    (" \"key with space\" value", "key with space", "value"),
    ("\"key with space\"  value", "key with space", "value"),
    (" \"key with space\"  value", "key with space", "value"),
];

/// `ParseQuotedValue`.
const PARSE_QUOTED_VALUE: &[(&str, &str, &str)] = &[
    ("key \"value\"", "key", "value"),
    (" key \"value\"", "key", "value"),
    ("key  \"value\"", "key", "value"),
    (" key  \"value\"", "key", "value"),
    (" key  \"value\" ", "key", "value"),
    ("key \"value with space\"", "key", "value with space"),
    (" key \"value with space\"", "key", "value with space"),
    ("key  \"value with space\"", "key", "value with space"),
    (" key  \"value with space\"", "key", "value with space"),
    (" key  \"value with space\" ", "key", "value with space"),
];

/// `ParseQuotedValueWithEscapedQuotes`.
const PARSE_QUOTED_VALUE_WITH_ESCAPED_QUOTES: &[(&str, &str, &str)] = &[
    ("key \"quotation mark: \\\"\"", "key", "quotation mark: \""),
    ("key \"quoted word: \\\"word\\\"\"", "key", "quoted word: \"word\""),
];

/// `ParseListOrDictionaryIgnoringValue`.
const PARSE_LIST_OR_DICTIONARY_IGNORING_VALUE: &[(&str, &str)] = &[
    ("Key Value\n[\n]", "Key"),
    ("Key Value\n{\n}", "Key"),
    ("Key // comment\n[\n]", "Key"),
    ("Key // comment\n{\n}", "Key"),
    ("\"Key\" Value\n[\n]", "Key"),
    ("\"Key\" Value\n{\n}", "Key"),
    ("\"Key\" // comment\n[\n]", "Key"),
    ("\"Key\" // comment\n{\n}", "Key"),
];

/// `ParseEmptyChildDictionary`.
const PARSE_EMPTY_CHILD_DICTIONARY: &[(&str, &str)] = &[
    ("key\n{\n}", "key"),
    ("key\n{\n\n}", "key"),
    ("key\n{\n\n\n}", "key"),
];

/// `ParseEmptyList`.
const PARSE_EMPTY_LIST: &[(&str, &str)] = &[
    ("key\n[\n]", "key"),
    ("key\n[\n\n]", "key"),
    ("key\n[\n\n\n]", "key"),
];

/// `ParseValuesInList`.
const PARSE_VALUES_IN_LIST: &[(&str, &str, &[&str])] = &[
    ("key\n[\nvalue1\n]", "key", &["value1"]),
    ("key\n[\nvalue1\nvalue2\n]", "key", &["value1", "value2"]),
    ("key\n[\nvalue1\nvalue2\nvalue3\n]", "key", &["value1", "value2", "value3"]),
    ("key\n[\nvalue with spaces\n]", "key", &["value with spaces"]),
    ("key\n[\nvalue1\nvalue with spaces\nvalue3\n]", "key", &["value1", "value with spaces", "value3"]),
    ("key\n[\n\"value with quotes\"\n]", "key", &["value with quotes"]),
    ("key\n[\nvalue1\n\"value with quotes\"\nvalue3\n]", "key", &["value1", "value with quotes", "value3"]),
];

/// `ParseListValuesWithEmptyLines`.
const PARSE_LIST_VALUES_WITH_EMPTY_LINES: &[(&str, &str, &[&str])] = &[
    ("key\n[\n\nvalue\n]", "key", &["value"]),
    ("key\n[\nvalue\n\n]", "key", &["value"]),
    ("key\n[\n\nvalue\n\n]", "key", &["value"]),
    ("key\n[\n\nvalue1\nvalue2\n\n]", "key", &["value1", "value2"]),
    ("key\n[\n\nvalue1\n\nvalue2\n\n]", "key", &["value1", "value2"]),
];

/// `ParseEmptyChildList`.
const PARSE_EMPTY_CHILD_LIST: &[(&str, &str)] = &[
    ("key\n[\n[\n]\n]", "key"),
];

/// `ParseMultipleChildLists`.
const PARSE_MULTIPLE_CHILD_LISTS: &[(&str, &str)] = &[
    ("key\n[\n[\n]\n[\n]\n]", "key"),
];

/// `ParseListWithMixedTypes`.
const PARSE_LIST_WITH_MIXED_TYPES: &[(&str, &[DatNodeType])] = &[
    ("list\n[\nvalue\n{\n}\n]", &[DatNodeType::Value, DatNodeType::Dictionary]),
    ("list\n[\nvalue\n[\n]\n]", &[DatNodeType::Value, DatNodeType::List]),
    ("list\n[\n{\n}\n[\n]\n]", &[DatNodeType::Dictionary, DatNodeType::List]),
    ("list\n[\n[\n]\n{\n}\n]", &[DatNodeType::List, DatNodeType::Dictionary]),
    ("list\n[\nvalue1\n{\n}\nvalue2\n]", &[DatNodeType::Value, DatNodeType::Dictionary, DatNodeType::Value]),
    ("list\n[\nvalue1\n[\n]\nvalue2\n]", &[DatNodeType::Value, DatNodeType::List, DatNodeType::Value]),
    ("list\n[\nvalue\n{\n}\n[\n]\n]", &[DatNodeType::Value, DatNodeType::Dictionary, DatNodeType::List]),
    ("list\n[\nvalue1\n{\n}\n[\n]\nvalue2\n]", &[DatNodeType::Value, DatNodeType::Dictionary, DatNodeType::List, DatNodeType::Value]),
    ("list\n[\nvalue1\n[\n]\n{\n}\nvalue2\n]", &[DatNodeType::Value, DatNodeType::List, DatNodeType::Dictionary, DatNodeType::Value]),
];

/// `ParseChildDictionaryValuesWithEmptyLines`.
const PARSE_CHILD_DICTIONARY_VALUES_WITH_EMPTY_LINES: &[(&str, &str, &[&str], &[&str])] = &[
    ("key\n{\n\nkey1 value1\n}", "key", &["key1"], &["value1"]),
    ("key\n{\nkey1 value1\n\n}", "key", &["key1"], &["value1"]),
    ("key\n{\n\nkey1 value1\n\n}", "key", &["key1"], &["value1"]),
    ("key\n{\n\nkey1 value1\nkey2 value2\n\n}", "key", &["key1", "key2"], &["value1", "value2"]),
    ("key\n{\n\nkey1 value1\n\nkey2 value2\n\n}", "key", &["key1", "key2"], &["value1", "value2"]),
    ("key\n{\n\nkey1 value1\n\n\nkey2 value2\n\n}", "key", &["key1", "key2"], &["value1", "value2"]),
];

/// `ParseValueInChildDictionary`.
const PARSE_VALUE_IN_CHILD_DICTIONARY: &[(&str, &str, &str, &str)] = &[
    ("key1\n{\nkey2 value\n}", "key1", "key2", "value"),
    ("key1\n{\nkey2 value with spaces\n}", "key1", "key2", "value with spaces"),
    ("key1\n{\nkey2 \"value with quotes\"\n}", "key1", "key2", "value with quotes"),
    ("key1\n{\nkey2 value2\nkey3 value3\nkey4 value4\n}", "key1", "key3", "value3"),
];

/// `ParseEmptyGrandchildDictionary`.
const PARSE_EMPTY_GRANDCHILD_DICTIONARY: &[(&str, &str, &str)] = &[
    ("key1\n{key2\n{\n}\n}", "key1", "key2"),
];

/// `ParseValueInGrandchildDictionary`.
const PARSE_VALUE_IN_GRANDCHILD_DICTIONARY: &[(&str, &str, &str, &str, &str)] = &[
    ("key1\n{key2\n{\nkey3 value\n}\n}", "key1", "key2", "key3", "value"),
    ("key1\n{key2\n{\nkey3 value with spaces\n}\n}", "key1", "key2", "key3", "value with spaces"),
    ("key1\n{key2\n{\nkey3 \"value with quotes\"\n}\n}", "key1", "key2", "key3", "value with quotes"),
    ("key1\n{key2\n{\nkey3 value3\nkey4 value4\nkey5 value5\n}\n}", "key1", "key2", "key4", "value4"),
];

/// `ParseEscapedNewLine`.
const PARSE_ESCAPED_NEW_LINE: &[(&str, &str)] = &[
    ("key first\\nsecond", "first\nsecond"),
    ("key \"first\\nsecond\"", "first\nsecond"),
];

/// `ParseSafeV1Asset`.
const PARSE_SAFE_V1_ASSET: &[&str] = &[
    concat!(
        "\r\n",
        "\t\tType Item\r\n",
        "\t\tID 342\r\n",
        "\t\tGUID 4d362f61e4324509844feedd1a69ae19\r\n",
        "\r\n",
        "\t\tFlag1\r\n",
        "\r\n",
        "\t\tEnableSomething true\r\n",
        "\t",
    ),
    concat!(
        "\r\n",
        "\t\t// comment\r\n",
        "\t\tType Item\r\n",
        "\t\tID 342\r\n",
        "\t\t// comment\r\n",
        "\t\t// comment\r\n",
        "\t\tGUID 4d362f61e4324509844feedd1a69ae19\r\n",
        "\t\t// comment\r\n",
        "\r\n",
        "\t\t// comment\r\n",
        "\t\tFlag1\r\n",
        "\r\n",
        "\t\tEnableSomething true\r\n",
        "\t\t// comment\r\n",
        "\t",
    ),
];

/// `ParseSafeV2Asset`.
const PARSE_SAFE_V2_ASSET: &[&str] = &[
    concat!(
        "\r\n",
        "\t\t\"Metadata\"\r\n",
        "\t\t{\r\n",
        "\t\t\t\"GUID\" \"4d362f61e4324509844feedd1a69ae19\"\r\n",
        "\t\t\t\"Type\" \"SDG.Unturned.ItemAsset\"\r\n",
        "\t\t}\r\n",
        "\t\t\"Asset\"\r\n",
        "\t\t{\r\n",
        "\t\t\t\"Name\" \"Bear+Dog\"\r\n",
        "\t\t\t\"Struct\"\r\n",
        "\t\t\t{\r\n",
        "\t\t\t\t\"X\" \"1\"\r\n",
        "\t\t\t\t\"Y\" \"2\"\r\n",
        "\t\t\t}\r\n",
        "\t\t\t\"Array\"\r\n",
        "\t\t\t[\r\n",
        "\t\t\t\t{\r\n",
        "\t\t\t\t\t\"A\" \"3\"\r\n",
        "\t\t\t\t\t\"B\" \"4\"\r\n",
        "\t\t\t\t}\r\n",
        "\t\t\t\t{\r\n",
        "\t\t\t\t\t\"A\" \"5\"\r\n",
        "\t\t\t\t\t\"B\" \"6\"\r\n",
        "\t\t\t\t}\r\n",
        "\t\t\t]\r\n",
        "\t\t}\r\n",
        "\t",
    ),
    concat!(
        "\r\n",
        "\t\t// comment\r\n",
        "\t\t// comment\r\n",
        "\t\t\"Metadata\" // comment\r\n",
        "\t\t{ // comment\r\n",
        "\t\t\t\"GUID\" \"4d362f61e4324509844feedd1a69ae19\" // comment\r\n",
        "\t\t\t\"Type\" \"SDG.Unturned.ItemAsset\" // comment\r\n",
        "\t\t} // comment\r\n",
        "\t\t// comment\r\n",
        "\t\t\"Asset\" // comment\r\n",
        "\t\t{ // comment\r\n",
        "\t\t\t\"Name\" \"Bear+Dog\" // comment\r\n",
        "\t\t\t\"Struct\" // comment\r\n",
        "\t\t\t{ // comment\r\n",
        "\t\t\t\t\"X\" \"1\" // comment\r\n",
        "\t\t\t\t\"Y\" \"2\" // comment\r\n",
        "\t\t\t} // comment\r\n",
        "\t\t\t\"Array\" // comment\r\n",
        "\t\t\t[ // comment\r\n",
        "\t\t\t\t// comment\r\n",
        "\t\t\t\t\r\n",
        "\t\t\t\t// comment\r\n",
        "\t\t\t\t{ // comment\r\n",
        "\t\t\t\t\t\"A\" \"3\" // comment\r\n",
        "\t\t\t\t\t\"B\" \"4\" // comment\r\n",
        "\t\t\t\t} // comment\r\n",
        "\t\t\t\t// comment\r\n",
        "\t\t\t\t{ // comment\r\n",
        "\t\t\t\t\t\"A\" \"5\" // comment\r\n",
        "\t\t\t\t\t\"B\" \"6\" // comment\r\n",
        "\t\t\t\t} // comment\r\n",
        "\t\t\t] // comment\r\n",
        "\t\t} // comment\r\n",
        "\t\t// comment\r\n",
        "\t",
    ),
    concat!(
        "\r\n",
        "\t\t\"Metadata\",\r\n",
        "\t\t{,\r\n",
        "\t\t\t\"GUID\" \"4d362f61e4324509844feedd1a69ae19\",\r\n",
        "\t\t\t\"Type\" \"SDG.Unturned.ItemAsset\",\r\n",
        "\t\t},\r\n",
        "\t\t\"Asset\",\r\n",
        "\t\t{,\r\n",
        "\t\t\t\"Name\" \"Bear+Dog\",\r\n",
        "\t\t\t\"Struct\",\r\n",
        "\t\t\t{,\r\n",
        "\t\t\t\t\"X\" \"1\",\r\n",
        "\t\t\t\t\"Y\" \"2\",\r\n",
        "\t\t\t},\r\n",
        "\t\t\t\"Array\",\r\n",
        "\t\t\t[,\r\n",
        "\t\t\t\t{,\r\n",
        "\t\t\t\t\t\"A\" \"3\",\r\n",
        "\t\t\t\t\t\"B\" \"4\",\r\n",
        "\t\t\t\t},\r\n",
        "\t\t\t\t{,\r\n",
        "\t\t\t\t\t\"A\" \"5\",\r\n",
        "\t\t\t\t\t\"B\" \"6\",\r\n",
        "\t\t\t\t},\r\n",
        "\t\t\t],\r\n",
        "\t\t},\r\n",
        "\t",
    ),
];

/// `ParseDictionaryOnSameLineAsValue`.
const PARSE_DICTIONARY_ON_SAME_LINE_AS_VALUE: &[(&str, &str, &str)] = &[
    ("key {", "key", "{"),
    ("key {\n", "key", "{"),
    ("key  {", "key", "{"),
    ("key  {\n", "key", "{"),
    ("key { \n", "key", "{ "),
    ("key {}", "key", "{}"),
    ("key {}\n", "key", "{}"),
    ("key { }", "key", "{ }"),
    ("key { }\n", "key", "{ }"),
];

/// `ParseListOnSameLineAsValue`.
const PARSE_LIST_ON_SAME_LINE_AS_VALUE: &[(&str, &str, &str)] = &[
    ("key [", "key", "["),
    ("key [\n", "key", "["),
    ("key  [", "key", "["),
    ("key  [\n", "key", "["),
    ("key [ \n", "key", "[ "),
    ("key []", "key", "[]"),
    ("key []\n", "key", "[]"),
    ("key [ ]", "key", "[ ]"),
    ("key [ ]\n", "key", "[ ]"),
];

/// `IgnoreCommasAfterValuesInList`.
const IGNORE_COMMAS_AFTER_VALUES_IN_LIST: &[(&str, &[&str])] = &[
    ("list\n[\n\"value\",\n]", &["value"]),
    ("list\n[\n\"value1\",\n\"value2\"\n]", &["value1", "value2"]),
    ("list\n[\n\"value1\",\n\"value2\",\n]", &["value1", "value2"]),
];

/// `IgnoreCommasAfterDictionariesInList`.
const IGNORE_COMMAS_AFTER_DICTIONARIES_IN_LIST: &[(&str, &[&str])] = &[
    ("list\n[\n{\nkey value\n},\n]", &["value"]),
    ("list\n[\n{\nkey value1\n},\n{\nkey value2\n}\n]", &["value1", "value2"]),
    ("list\n[\n{\nkey value1\n},\n{\nkey value2\n},\n]", &["value1", "value2"]),
];

/// `IgnoreCommasAfterValuesInDictionary`.
const IGNORE_COMMAS_AFTER_VALUES_IN_DICTIONARY: &[(&str, &[&str], &[&str])] = &[
    ("\"key1\" \"value1\",", &["key1"], &["value1"]),
    ("\"key1\" \"value1\",\n", &["key1"], &["value1"]),
    ("\"key1\" \"value1\",\n\"key2\" \"value2\"", &["key1", "key2"], &["value1", "value2"]),
    ("\"key1\" \"value1\",\n\"key2\" \"value2\",", &["key1", "key2"], &["value1", "value2"]),
    ("\"key1\" \"value1\",\n\"key2\" \"value2\",\n", &["key1", "key2"], &["value1", "value2"]),
];

/// `ParseValidEscapeSequences`.
const PARSE_VALID_ESCAPE_SEQUENCES: &[(&str, &str)] = &[
    ("key \\n", "\n"),
    ("key \\t", "\t"),
    ("key \\\\", "\\"),
    ("key \"\\n\"", "\n"),
    ("key \"\\t\"", "\t"),
    ("key \"\\\\\"", "\\"),
    ("key \"\\\"\"", "\""),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("IgnoreRootComments", IGNORE_ROOT_COMMENTS.len()),
    ("IgnoreRootCommentsBeforeKey", IGNORE_ROOT_COMMENTS_BEFORE_KEY.len()),
    ("IgnoreDictionaryComments", IGNORE_DICTIONARY_COMMENTS.len()),
    ("IgnoreDictionaryCommentsBeforeKey", IGNORE_DICTIONARY_COMMENTS_BEFORE_KEY.len()),
    ("IgnoreDictionaryCommentsAfterKey", IGNORE_DICTIONARY_COMMENTS_AFTER_KEY.len()),
    ("IgnoreCommentsOnSameLineAsDictionaryValue", IGNORE_COMMENTS_ON_SAME_LINE_AS_DICTIONARY_VALUE.len()),
    ("IgnoreListComments", IGNORE_LIST_COMMENTS.len()),
    ("IgnoreListCommentsBeforeValue", IGNORE_LIST_COMMENTS_BEFORE_VALUE.len()),
    ("IgnoreListCommentsAfterValue", IGNORE_LIST_COMMENTS_AFTER_VALUE.len()),
    ("IgnoreCommentsOnSameLineAsListValue", IGNORE_COMMENTS_ON_SAME_LINE_AS_LIST_VALUE.len()),
    ("ParseEmptyRootDictionary", PARSE_EMPTY_ROOT_DICTIONARY.len()),
    ("ParseValueBackslash", PARSE_VALUE_BACKSLASH.len()),
    ("ParseMultiLineString", PARSE_MULTI_LINE_STRING.len()),
    ("ParseFlag", PARSE_FLAG.len()),
    ("ParseMultipleFlags", PARSE_MULTIPLE_FLAGS.len()),
    ("ParseValueWithTrailingSpaces", PARSE_VALUE_WITH_TRAILING_SPACES.len()),
    ("ParseSingleLineWithoutQuotes", PARSE_SINGLE_LINE_WITHOUT_QUOTES.len()),
    ("ParseMultiLineWithoutQuotes", PARSE_MULTI_LINE_WITHOUT_QUOTES.len()),
    ("ParseQuotedKey", PARSE_QUOTED_KEY.len()),
    ("ParseQuotedValue", PARSE_QUOTED_VALUE.len()),
    ("ParseQuotedValueWithEscapedQuotes", PARSE_QUOTED_VALUE_WITH_ESCAPED_QUOTES.len()),
    ("ParseListOrDictionaryIgnoringValue", PARSE_LIST_OR_DICTIONARY_IGNORING_VALUE.len()),
    ("ParseEmptyChildDictionary", PARSE_EMPTY_CHILD_DICTIONARY.len()),
    ("ParseEmptyList", PARSE_EMPTY_LIST.len()),
    ("ParseValuesInList", PARSE_VALUES_IN_LIST.len()),
    ("ParseListValuesWithEmptyLines", PARSE_LIST_VALUES_WITH_EMPTY_LINES.len()),
    ("ParseEmptyChildList", PARSE_EMPTY_CHILD_LIST.len()),
    ("ParseMultipleChildLists", PARSE_MULTIPLE_CHILD_LISTS.len()),
    ("ParseListWithMixedTypes", PARSE_LIST_WITH_MIXED_TYPES.len()),
    ("ParseChildDictionaryValuesWithEmptyLines", PARSE_CHILD_DICTIONARY_VALUES_WITH_EMPTY_LINES.len()),
    ("ParseValueInChildDictionary", PARSE_VALUE_IN_CHILD_DICTIONARY.len()),
    ("ParseEmptyGrandchildDictionary", PARSE_EMPTY_GRANDCHILD_DICTIONARY.len()),
    ("ParseValueInGrandchildDictionary", PARSE_VALUE_IN_GRANDCHILD_DICTIONARY.len()),
    ("ParseEscapedNewLine", PARSE_ESCAPED_NEW_LINE.len()),
    ("ParseSafeV1Asset", PARSE_SAFE_V1_ASSET.len()),
    ("ParseSafeV2Asset", PARSE_SAFE_V2_ASSET.len()),
    ("ParseDictionaryOnSameLineAsValue", PARSE_DICTIONARY_ON_SAME_LINE_AS_VALUE.len()),
    ("ParseListOnSameLineAsValue", PARSE_LIST_ON_SAME_LINE_AS_VALUE.len()),
    ("IgnoreCommasAfterValuesInList", IGNORE_COMMAS_AFTER_VALUES_IN_LIST.len()),
    ("IgnoreCommasAfterDictionariesInList", IGNORE_COMMAS_AFTER_DICTIONARIES_IN_LIST.len()),
    ("IgnoreCommasAfterValuesInDictionary", IGNORE_COMMAS_AFTER_VALUES_IN_DICTIONARY.len()),
    ("ParseValidEscapeSequences", PARSE_VALID_ESCAPE_SEQUENCES.len()),
    ("SkipBOM", 1),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 217, "{TABLES:?}");
}

/// The tables whose rows each check one `GetString(key)` on the root.
fn assert_key_values(rows: &[(&str, &str, &str)]) {
    for &(input, expected_key, expected_value) in rows {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_string(expected_value, get_string(tree.root(), expected_key), input);
    }
}

/// The tables whose rows each check `GetString("key")` on the root.
fn assert_key_value(rows: &[(&str, &str)]) {
    for &(input, expected_value) in rows {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_string(expected_value, get_string(tree.root(), "key"), input);
    }
}

/// `ParseValuesInList` and `ParseListValuesWithEmptyLines`.
fn assert_list_values(rows: &[(&str, &str, &[&str])]) {
    for &(input, expected_key, expected_values) in rows {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), expected_key), "TryGetList result", input);
        assert_eq!(expected_values.len(), list.len(), "{input:?}");
        for (index, &expected_value) in expected_values.iter().enumerate() {
            assert_string(expected_value, list_get_string(list, index as i32), input);
        }
    }
}

#[test]
fn ignore_root_comments() {
    for &input in IGNORE_ROOT_COMMENTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(0, tree.root().len(), "{input:?}");
    }
}

#[test]
fn ignore_root_comments_before_key() {
    for &input in IGNORE_ROOT_COMMENTS_BEFORE_KEY {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(1, tree.root().len(), "{input:?}");
        assert_string("value", get_string(tree.root(), "key"), input);
    }
}

#[test]
fn ignore_dictionary_comments() {
    for &input in IGNORE_DICTIONARY_COMMENTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(1, tree.root().len(), "{input:?}");
        let child_dictionary = found(get_dictionary(tree.root(), "key"), "GetDictionary(key)", input);
        assert_eq!(0, child_dictionary.len(), "{input:?}");
    }
}

/// `IgnoreDictionaryCommentsBeforeKey` and `IgnoreDictionaryCommentsAfterKey`.
fn assert_one_child_value(rows: &[&str]) {
    for &input in rows {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(1, tree.root().len(), "{input:?}");
        let child_dictionary = found(get_dictionary(tree.root(), "key"), "GetDictionary(key)", input);
        assert_eq!(1, child_dictionary.len(), "{input:?}");
        assert_string("value", get_string(child_dictionary, "key"), input);
    }
}

#[test]
fn ignore_dictionary_comments_before_key() {
    assert_one_child_value(IGNORE_DICTIONARY_COMMENTS_BEFORE_KEY);
}

#[test]
fn ignore_dictionary_comments_after_key() {
    assert_one_child_value(IGNORE_DICTIONARY_COMMENTS_AFTER_KEY);
}

/// Same-line comments are only supported for quoted values (required for .dat backwards compatibility).
#[test]
fn ignore_comments_on_same_line_as_dictionary_value() {
    assert_key_values(IGNORE_COMMENTS_ON_SAME_LINE_AS_DICTIONARY_VALUE);
}

#[test]
fn ignore_list_comments() {
    for &input in IGNORE_LIST_COMMENTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(1, tree.root().len(), "{input:?}");
        let list = found(get_list(tree.root(), "key"), "GetList(key)", input);
        assert_eq!(0, list.len(), "{input:?}");
    }
}

/// `IgnoreListCommentsBeforeValue` and `IgnoreListCommentsAfterValue`.
fn assert_one_list_value(rows: &[&str]) {
    for &input in rows {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(1, tree.root().len(), "{input:?}");
        let list = found(get_list(tree.root(), "key"), "GetList(key)", input);
        assert_eq!(1, list.len(), "{input:?}");
        assert_string("value", list_get_string(list, 0), input);
    }
}

#[test]
fn ignore_list_comments_before_value() {
    assert_one_list_value(IGNORE_LIST_COMMENTS_BEFORE_VALUE);
}

#[test]
fn ignore_list_comments_after_value() {
    assert_one_list_value(IGNORE_LIST_COMMENTS_AFTER_VALUE);
}

/// Same-line comments are only supported for quoted values.
#[test]
fn ignore_comments_on_same_line_as_list_value() {
    for &(input, expected_value) in IGNORE_COMMENTS_ON_SAME_LINE_AS_LIST_VALUE {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(1, tree.root().len(), "{input:?}");
        let list = found(get_list(tree.root(), "key"), "GetList(key)", input);
        assert_eq!(1, list.len(), "{input:?}");
        assert_string(expected_value, list_get_string(list, 0), input);
    }
}

#[test]
fn parse_empty_root_dictionary() {
    for &input in PARSE_EMPTY_ROOT_DICTIONARY {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(0, tree.root().len(), "{input:?}");
    }
}

#[test]
fn parse_value_backslash() {
    assert_key_values(PARSE_VALUE_BACKSLASH);
}

#[test]
fn parse_multi_line_string() {
    assert_key_values(PARSE_MULTI_LINE_STRING);
}

/// Some old .dat options check ContainsKey rather than true/false.
#[test]
fn parse_flag() {
    for &(input, expected_key) in PARSE_FLAG {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert!(tree.root().contains_key(expected_key), "{input:?}");
    }
}

#[test]
fn parse_multiple_flags() {
    for &(input, expected_key1, expected_key2) in PARSE_MULTIPLE_FLAGS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert!(tree.root().contains_key(expected_key1), "ContainsKey({expected_key1}) for {input:?}");
        assert!(tree.root().contains_key(expected_key2), "ContainsKey({expected_key2}) for {input:?}");
    }
}

#[test]
fn parse_value_with_trailing_spaces() {
    assert_key_values(PARSE_VALUE_WITH_TRAILING_SPACES);
}

#[test]
fn parse_single_line_without_quotes() {
    assert_key_values(PARSE_SINGLE_LINE_WITHOUT_QUOTES);
}

#[test]
fn parse_multi_line_without_quotes() {
    for &(input, expected_key1, expected_value1, expected_key2, expected_value2) in PARSE_MULTI_LINE_WITHOUT_QUOTES {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_string(expected_value1, get_string(tree.root(), expected_key1), input);
        assert_string(expected_value2, get_string(tree.root(), expected_key2), input);
    }
}

#[test]
fn parse_quoted_key() {
    assert_key_values(PARSE_QUOTED_KEY);
}

#[test]
fn parse_quoted_value() {
    assert_key_values(PARSE_QUOTED_VALUE);
}

#[test]
fn parse_quoted_value_with_escaped_quotes() {
    assert_key_values(PARSE_QUOTED_VALUE_WITH_ESCAPED_QUOTES);
}

/// Nelson 2025-04-14: even if a key is assigned a value, a list or dictionary on the next line takes precedence.
/// Caught because you can't comment flags: `Flag // comment` parses the comment as the flag's value.
#[test]
fn parse_list_or_dictionary_ignoring_value() {
    for &(input, key) in PARSE_LIST_OR_DICTIONARY_IGNORING_VALUE {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let node = found(tree.root().try_get_node(key), "rootDictionary[key]", input);
        assert_ne!(DatNodeType::Value, node.node_type(), "{input:?}");
    }
}

#[test]
fn parse_empty_child_dictionary() {
    for &(input, expected_key) in PARSE_EMPTY_CHILD_DICTIONARY {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let child_dictionary = found(get_dictionary(tree.root(), expected_key), "TryGetDictionary result", input);
        assert_eq!(0, child_dictionary.len(), "{input:?}");
    }
}

#[test]
fn parse_empty_list() {
    for &(input, expected_key) in PARSE_EMPTY_LIST {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), expected_key), "TryGetList result", input);
        assert_eq!(0, list.len(), "{input:?}");
    }
}

#[test]
fn parse_values_in_list() {
    assert_list_values(PARSE_VALUES_IN_LIST);
}

#[test]
fn parse_list_values_with_empty_lines() {
    assert_list_values(PARSE_LIST_VALUES_WITH_EMPTY_LINES);
}

#[test]
fn parse_empty_child_list() {
    for &(input, expected_root_key) in PARSE_EMPTY_CHILD_LIST {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), expected_root_key), "TryGetList result", input);
        found(list.try_get_node(0).and_then(|node| node.as_list()), "childList", input);
    }
}

#[test]
fn parse_multiple_child_lists() {
    for &(input, expected_root_key) in PARSE_MULTIPLE_CHILD_LISTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), expected_root_key), "TryGetList result", input);
        assert_eq!(2, list.len(), "{input:?}");
        found(list.try_get_node(0).and_then(|node| node.as_list()), "list[0] as IDatList", input);
        found(list.try_get_node(1).and_then(|node| node.as_list()), "list[1] as IDatList", input);
    }
}

/// Mis-configured files with a mix of values, dictionaries and lists in one list should be handled properly.
#[test]
fn parse_list_with_mixed_types() {
    for &(input, expected_types) in PARSE_LIST_WITH_MIXED_TYPES {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "list"), "TryGetList result", input);
        assert_eq!(expected_types.len(), list.len(), "{input:?}");
        for (index, &expected_type) in expected_types.iter().enumerate() {
            let node = found(list.try_get_node(index as i32), "list[index]", input);
            assert_eq!(expected_type, node.node_type(), "list[{index}] for {input:?}");
        }
    }
}

#[test]
fn parse_child_dictionary_values_with_empty_lines() {
    for &(input, expected_root_key, expected_child_keys, expected_child_values) in
        PARSE_CHILD_DICTIONARY_VALUES_WITH_EMPTY_LINES
    {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let child_dictionary =
            found(get_dictionary(tree.root(), expected_root_key), "TryGetDictionary result", input);
        assert_eq!(expected_child_keys.len(), expected_child_values.len());
        assert_eq!(expected_child_keys.len(), child_dictionary.len(), "{input:?}");
        for (&key, &value) in expected_child_keys.iter().zip(expected_child_values) {
            assert_string(value, get_string(child_dictionary, key), input);
        }
    }
}

#[test]
fn parse_value_in_child_dictionary() {
    for &(input, expected_root_key, expected_child_key, expected_child_value) in PARSE_VALUE_IN_CHILD_DICTIONARY {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let child_dictionary =
            found(get_dictionary(tree.root(), expected_root_key), "TryGetDictionary result", input);
        assert_string(expected_child_value, get_string(child_dictionary, expected_child_key), input);
    }
}

#[test]
fn parse_empty_grandchild_dictionary() {
    for &(input, expected_root_key, expected_child_key) in PARSE_EMPTY_GRANDCHILD_DICTIONARY {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let child_dictionary =
            found(get_dictionary(tree.root(), expected_root_key), "rootDictionary.TryGetDictionary result", input);
        found(get_dictionary(child_dictionary, expected_child_key), "childDictionary.TryGetDictionary result", input);
    }
}

#[test]
fn parse_value_in_grandchild_dictionary() {
    for &(input, expected_root_key, expected_child_key, expected_grandchild_key, expected_grandchild_value) in
        PARSE_VALUE_IN_GRANDCHILD_DICTIONARY
    {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let child_dictionary =
            found(get_dictionary(tree.root(), expected_root_key), "rootDictionary.TryGetDictionary result", input);
        let grandchild_dictionary = found(
            get_dictionary(child_dictionary, expected_child_key),
            "childDictionary.TryGetDictionary result",
            input,
        );
        assert_string(expected_grandchild_value, get_string(grandchild_dictionary, expected_grandchild_key), input);
    }
}

#[test]
fn parse_escaped_new_line() {
    assert_key_value(PARSE_ESCAPED_NEW_LINE);
}

/// `new System.Guid("4d362f61e4324509844feedd1a69ae19")`.
fn asset_guid() -> Guid {
    Guid::parse(&NetString::from("4d362f61e4324509844feedd1a69ae19")).expect("a valid GUID")
}

/// Keys and values from a typical old vanilla .dat file.
#[test]
fn parse_safe_v1_asset() {
    for &contents in PARSE_SAFE_V1_ASSET {
        let (parser, tree) = parse(contents);
        assert_no_error(&parser, contents);
        let dictionary = tree.root();
        assert_string("Item", get_string(dictionary, "Type"), contents);
        assert_eq!(342, dictionary.parse_uint16("ID", 0), "{contents:?}");
        assert_eq!(asset_guid(), dictionary.parse_guid("GUID", Guid::default()), "{contents:?}");
        assert!(dictionary.contains_key("Flag1"), "{contents:?}");
        assert!(dictionary.parse_bool("EnableSomething", false), "{contents:?}");
    }
}

/// The contents of a typical vanilla .asset file.
#[test]
fn parse_safe_v2_asset() {
    for &contents in PARSE_SAFE_V2_ASSET {
        let (parser, tree) = parse(contents);
        assert_no_error(&parser, contents);
        let root_dictionary = tree.root();

        let metadata = found(get_dictionary(root_dictionary, "Metadata"), "metadata", contents);
        assert_eq!(asset_guid(), metadata.parse_guid("GUID", Guid::default()), "{contents:?}");
        assert_string("SDG.Unturned.ItemAsset", get_string(metadata, "Type"), contents);

        let asset = found(get_dictionary(root_dictionary, "Asset"), "asset", contents);
        assert_string("Bear+Dog", get_string(asset, "Name"), contents);

        let substruct = found(get_dictionary(asset, "Struct"), "substruct", contents);
        assert_eq!(1, substruct.parse_int32("X", 0), "{contents:?}");
        assert_eq!(2, substruct.parse_int32("Y", 0), "{contents:?}");

        let list = found(get_list(asset, "Array"), "list", contents);
        let list_dict0 = found(list_get_dictionary(list, 0), "listDict0", contents);
        let list_dict1 = found(list_get_dictionary(list, 1), "listDict1", contents);
        assert_eq!(3, list_dict0.parse_int32("A", 0), "{contents:?}");
        assert_eq!(4, list_dict0.parse_int32("B", 0), "{contents:?}");
        assert_eq!(5, list_dict1.parse_int32("A", 0), "{contents:?}");
        assert_eq!(6, list_dict1.parse_int32("B", 0), "{contents:?}");
    }
}

/// For .dat file backwards compatibility '{' should be treated as a value.
#[test]
fn parse_dictionary_on_same_line_as_value() {
    assert_key_values(PARSE_DICTIONARY_ON_SAME_LINE_AS_VALUE);
}

/// For .dat file backwards compatibility '[' should be treated as a value.
#[test]
fn parse_list_on_same_line_as_value() {
    assert_key_values(PARSE_LIST_ON_SAME_LINE_AS_VALUE);
}

/// Many "v2" assets in curated maps have unnecessary commas, so they need to be excluded from list entries.
#[test]
fn ignore_commas_after_values_in_list() {
    for &(input, expected_values) in IGNORE_COMMAS_AFTER_VALUES_IN_LIST {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "list"), "TryGetList result", input);
        assert_eq!(expected_values.len(), list.len(), "{input:?}");
        for (index, &expected_value) in expected_values.iter().enumerate() {
            assert_string(expected_value, list_get_string(list, index as i32), input);
        }
    }
}

#[test]
fn ignore_commas_after_dictionaries_in_list() {
    for &(input, expected_values) in IGNORE_COMMAS_AFTER_DICTIONARIES_IN_LIST {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "list"), "TryGetList result", input);
        assert_eq!(expected_values.len(), list.len(), "{input:?}");
        for (index, &expected_value) in expected_values.iter().enumerate() {
            let dictionary = found(list_get_dictionary(list, index as i32), "list.GetDictionary(index)", input);
            assert_string(expected_value, get_string(dictionary, "key"), input);
        }
    }
}

/// Many "v2" assets in curated maps have unnecessary commas, so they need to be excluded from dictionary keys.
#[test]
fn ignore_commas_after_values_in_dictionary() {
    for &(input, expected_keys, expected_values) in IGNORE_COMMAS_AFTER_VALUES_IN_DICTIONARY {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        assert_eq!(expected_keys.len(), expected_values.len());
        assert_eq!(expected_keys.len(), tree.root().len(), "{input:?}");
        for (&key, &value) in expected_keys.iter().zip(expected_values) {
            assert_string(value, get_string(tree.root(), key), input);
        }
    }
}

/// 2023-05-27: 3.23.7.0 added '\n' handling for unquoted strings, which broke mods using '\' in file paths. These
/// are the supported escape sequences; `invalid_escape_sequence` covers the rest.
#[test]
fn parse_valid_escape_sequences() {
    assert_key_value(PARSE_VALID_ESCAPE_SEQUENCES);
}

/// 2023-07-12: main menu banner was broken because string included byte order mark.
///
/// The C# writes `Key Value` and a line break with a UTF-8 byte order mark into `new MemoryStream(new byte[32])`,
/// whose length stays 32, so `ToArray()` ends in zero bytes. They decode to NUL characters, which aren't whitespace,
/// so they form a second key.
#[test]
fn skip_bom() {
    let mut data = vec![0xEF, 0xBB, 0xBF];
    data.extend_from_slice(b"Key Value");
    data.extend_from_slice(NEW_LINE.as_bytes());
    data.resize(32, 0);
    assert_eq!(0xEF, data[0], "first byte");
    assert_eq!(0xBB, data[1], "second byte");
    assert_eq!(0xBF, data[2], "third byte");

    let mut parser = DatParser::new();
    let dictionary = parser.parse_bytes(&data);
    assert!(!parser.has_error(), "Error message: {:?}", parser.error_message());
    assert_string("Value", get_string(dictionary.root(), "Key"), "bytes");
    assert_eq!(2, dictionary.root().len());

    // Hack to easily put BOM characters in string.
    let string_with_bom = NetString::from_units(data.iter().map(|&byte| u16::from(byte)).collect());
    assert_eq!(0xEF, string_with_bom.as_units()[0], "first char");
    assert_eq!(0xBB, string_with_bom.as_units()[1], "second char");
    assert_eq!(0xBF, string_with_bom.as_units()[2], "third char");
    let string_dictionary = parser.parse_text(&string_with_bom);
    assert!(!parser.has_error(), "Error message: {:?}", parser.error_message());
    assert_string("Value", get_string(string_dictionary.root(), "Key"), "chars");
}

/// The pairs enumerate in insertion order. A duplicate key replaces the value where the first one was, and the key
/// keeps its first spelling.
#[test]
fn dictionary_enumerates_in_insertion_order() {
    let (_, tree) = parse("b 1\na 2\nB 3\nc 4");
    let pairs: Vec<(NetString, Option<NetString>)> = tree
        .root()
        .iter()
        .map(|(key, node)| (key.clone(), node.as_value().and_then(|value| value.value()).cloned()))
        .collect();
    let expected = [("b", "3"), ("a", "2"), ("c", "4")]
        .map(|(key, value)| (NetString::from(key), Some(NetString::from(value))));
    assert_eq!(pairs, expected);
}

/// `DatList.TryGetNode` finds nothing outside `0 .. Count`, and `GetValues` skips items that aren't values.
#[test]
fn list_lookups() {
    let (_, tree) = parse("list\n[\na\n{\n}\nb\n[\n]\n]");
    let list = found(get_list(tree.root(), "list"), "list", "list");
    assert_eq!(4, list.len());
    assert!(list.try_get_node(-1).is_none());
    assert!(list.try_get_node(4).is_none());
    assert!(list.try_get_node(i32::MIN).is_none());
    assert_string("b", list_get_string(list, 2), "list");

    let values: Vec<Option<NetString>> = list.values().map(|value| value.value().cloned()).collect();
    assert_eq!(values, [Some(NetString::from("a")), Some(NetString::from("b"))]);
    let types: Vec<DatNodeType> = list.iter().map(|node| node.node_type()).collect();
    assert_eq!(types, [DatNodeType::Value, DatNodeType::Dictionary, DatNodeType::Value, DatNodeType::List]);
}

/// Looking a node up again gives the same node, and other nodes differ.
#[test]
fn node_identity() {
    let (_, tree) = parse("a 1\nb 2");
    let a = found(tree.root().try_get_node("a"), "a", "a");
    let b = found(tree.root().try_get_node("b"), "b", "b");
    assert_eq!(a.id(), found(tree.root().try_get_node("A"), "A", "A").id());
    assert_ne!(a.id(), b.id());
    assert_ne!(a.id(), tree.root().id());
}

/// `DebugDumpToString`: dictionaries list `"key" = ` pairs and lists list `index = ` items, one tab deeper per
/// level, and values are quoted or `value(null)`. A child node dumps from indentation level 0.
#[test]
fn debug_dump_to_string() {
    let (_, tree) = parse("a 1\nd\n{\nk v\nl\n[\nx\n{\n}\n]\n}\nf");
    let lines = |lines: &[&str]| lines.iter().map(|line| format!("{line}{NEW_LINE}")).collect::<String>();

    let expected = lines(&[
        "{",
        "\t\"a\" = \"1\"",
        "\t\"d\" = {",
        "\t\t\"k\" = \"v\"",
        "\t\t\"l\" = [",
        "\t\t\t0 = \"x\"",
        "\t\t\t1 = {",
        "\t\t\t}",
        "\t\t]",
        "\t}",
        "\t\"f\" = value(null)",
        "}",
    ]);
    assert_eq!(NodeRef::Dictionary(tree.root()).debug_dump_to_string(), expected.as_str());

    let list = found(get_dictionary(tree.root(), "d").and_then(|d| get_list(d, "l")), "d/l", "dump");
    let expected = lines(&["[", "\t0 = \"x\"", "\t1 = {", "\t}", "]"]);
    assert_eq!(NodeRef::List(list).debug_dump_to_string(), expected.as_str());
}
