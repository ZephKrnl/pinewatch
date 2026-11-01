//! The list helpers: the six `[Test]` methods of `Assets/Tests/UnturnedDat/DatListTests.cs`. The C# builds its lists
//! in code; here they are parsed, with nodes of the same types in the same order. The C#'s `new DatValue()` items
//! have null text, which no parsed list item has, and the assertions don't look at it. `EnumerateValues` also puts a
//! null item in its list, which a parsed tree can't hold.

mod common;

use common::{found, parse};
use pinewatch_mono::NetString;

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("TryGetDictionary", 1),
    ("GetDictionary", 1),
    ("TryGetList", 1),
    ("GetList", 1),
    ("GetStrings", 1),
    ("EnumerateValues", 1),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 6, "{TABLES:?}");
}

/// `new DatList { new DatValue(), new DatDictionary(), new DatValue() }`.
const VALUE_DICTIONARY_VALUE: &str = "list\n[\na\n{\n}\nb\n]";

/// `new DatList { new DatValue(), new DatList(), new DatValue() }`.
const VALUE_LIST_VALUE: &str = "list\n[\na\n[\n]\nb\n]";

#[test]
fn try_get_dictionary() {
    let (_, tree) = parse(VALUE_DICTIONARY_VALUE);
    let list = found(common::get_list(tree.root(), "list"), "list", VALUE_DICTIONARY_VALUE);
    assert!(list.try_get_dictionary(0).is_none());
    assert!(list.try_get_dictionary(1).is_some());
    assert!(list.try_get_dictionary(2).is_none());
    assert!(list.try_get_dictionary(3).is_none());
}

/// `GetDictionary(index)` is `TryGetDictionary(index)` or null, which is what `try_get_dictionary` returns.
#[test]
fn get_dictionary() {
    let (_, tree) = parse(VALUE_DICTIONARY_VALUE);
    let list = found(common::get_list(tree.root(), "list"), "list", VALUE_DICTIONARY_VALUE);
    let found_at: Vec<bool> = (0..4).map(|index| list.try_get_dictionary(index).is_some()).collect();
    assert_eq!(found_at, [false, true, false, false]);
}

#[test]
fn try_get_list() {
    let (_, tree) = parse(VALUE_LIST_VALUE);
    let list = found(common::get_list(tree.root(), "list"), "list", VALUE_LIST_VALUE);
    assert!(list.try_get_list(0).is_none());
    assert!(list.try_get_list(1).is_some());
    assert!(list.try_get_list(2).is_none());
    assert!(list.try_get_list(3).is_none());
}

/// `GetList(index)` is `TryGetList(index)` or null.
#[test]
fn get_list() {
    let (_, tree) = parse(VALUE_LIST_VALUE);
    let list = found(common::get_list(tree.root(), "list"), "list", VALUE_LIST_VALUE);
    let found_at: Vec<bool> = (0..4).map(|index| list.try_get_list(index).is_some()).collect();
    assert_eq!(found_at, [false, true, false, false]);
}

#[test]
fn get_strings() {
    let values = ["abc", "def", "ghi"];
    let (_, tree) = parse("list\n[\nabc\ndef\nghi\n]");
    let list = found(common::get_list(tree.root(), "list"), "list", "strings");
    assert_eq!(values.len(), list.len());
    for (index, node) in list.iter().enumerate() {
        let literal = found(node.as_value(), "value as IDatValue", "strings");
        assert_eq!(Some(&NetString::from(values[index])), literal.value());
    }
}

#[test]
fn enumerate_values() {
    let input = "list\n[\n{\n}\nabc\ndef\n[\n]\n{\n}\nghi\n]";
    let (_, tree) = parse(input);
    let list = found(common::get_list(tree.root(), "list"), "list", input);
    let values: Vec<Option<&NetString>> = list.values().map(|value| value.value()).collect();
    let expected_values = ["abc", "def", "ghi"].map(NetString::from);
    assert_eq!(values, expected_values.iter().map(Some).collect::<Vec<_>>());
}

/// `TryGetString(index)` succeeds for a value item even when its text is null, and `GetString(index)` then returns
/// that text, not the default. Other items and indexes outside the list give the default.
#[test]
fn get_string_by_index() {
    let (_, tree) = parse("list\n[\nabc\n{\n}\n]");
    let list = found(common::get_list(tree.root(), "list"), "list", "by index");
    let abc = NetString::from("abc");
    let default = NetString::from("default");
    assert_eq!(Some(Some(&abc)), list.try_get_string(0));
    assert_eq!(Some(&abc), list.get_string(0, Some(&default)));
    for index in [1, 2, -1] {
        assert_eq!(None, list.try_get_string(index), "{index}");
        assert_eq!(Some(&default), list.get_string(index, Some(&default)), "{index}");
    }
    assert!(list.try_get_value(0).is_some());
    assert!(list.try_get_value(1).is_none());
}
