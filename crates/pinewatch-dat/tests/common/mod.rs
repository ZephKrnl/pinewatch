//! Helpers shared by the parser tests. The lookups are the C# tests' `DatDictionaryEx` and `DatListEx` calls,
//! written on the tree API with the same results.

#![allow(dead_code)]

use pinewatch_dat::{DatParser, DatTree, DictRef, ListRef, ValueRef};
use pinewatch_mono::{EnumInfo, EnumUnderlying, NetEnum, NetString};

/// `dictionary["key"] = new DatValue(input)`: a tree whose root holds `input` under `key`. It's parsed from
/// `key "…"` with `\` and `"` escaped, which reads back exactly `input`.
pub fn tree_with_value(input: &str) -> DatTree {
    let escaped = input.replace('\\', "\\\\").replace('"', "\\\"");
    let (parser, tree) = parse(&format!("key \"{escaped}\""));
    assert!(!parser.has_error(), "{:?}", parser.error_messages());
    tree
}

/// `EDatTestEnum` from the C# tests.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DatTestEnum(pub i32);

impl DatTestEnum {
    pub const NONE: Self = Self(0);
    pub const BEAR: Self = Self(1);
    pub const DOG: Self = Self(2);
}

impl NetEnum for DatTestEnum {
    const INFO: EnumInfo =
        EnumInfo { underlying: EnumUnderlying::I32, names: &["None", "Bear", "Dog"], values: &[0, 1, 2] };

    fn from_bits(bits: u64) -> Self {
        Self(bits as i32)
    }
}

/// `[System.Flags] EDatTestEnumFlags` from the C# tests, where `Flag2` is `2 << 1`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DatTestEnumFlags(pub i32);

impl DatTestEnumFlags {
    pub const NONE: Self = Self(0);
    pub const FLAG1: Self = Self(1 << 0);
    pub const FLAG2: Self = Self(2 << 1);

    /// `self | other`.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl NetEnum for DatTestEnumFlags {
    const INFO: EnumInfo =
        EnumInfo { underlying: EnumUnderlying::I32, names: &["None", "Flag1", "Flag2"], values: &[0, 1, 4] };

    fn from_bits(bits: u64) -> Self {
        Self(bits as i32)
    }
}

/// `new DatParser().Parse(input)`.
pub fn parse(input: &str) -> (DatParser, DatTree) {
    let mut parser = DatParser::new();
    let tree = parser.parse_str(input);
    (parser, tree)
}

/// `new DatParser { EnableMetadata = true }.Parse(input)`.
pub fn parse_with_metadata(input: &str) -> (DatParser, DatTree) {
    let mut parser = DatParser::new();
    parser.set_enable_metadata(true);
    let tree = parser.parse_str(input);
    (parser, tree)
}

/// `TryGetValue(key, out IDatValue value)`: the value under `key`, if that's what is there.
pub fn get_value<'t>(dictionary: DictRef<'t>, key: &str) -> Option<ValueRef<'t>> {
    dictionary.try_get_node(key)?.as_value()
}

/// `list.TryGetValue(index, out IDatValue value)`.
pub fn list_get_value<'t>(list: ListRef<'t>, index: i32) -> Option<ValueRef<'t>> {
    list.try_get_node(index)?.as_value()
}

/// `Assert.IsFalse(parser.HasError, …)`.
#[track_caller]
pub fn assert_no_error(parser: &DatParser, input: &str) {
    assert!(!parser.has_error(), "parser.HasError for {input:?}, Error message: {:?}", parser.error_message());
}

/// `Assert.IsTrue(parser.HasError, …)`.
#[track_caller]
pub fn assert_error(parser: &DatParser, input: &str) {
    assert!(parser.has_error(), "parser.HasError for {input:?}");
}

/// `Assert.AreEqual(expected, actual)` for a string.
#[track_caller]
pub fn assert_string(expected: &str, actual: Option<&NetString>, input: &str) {
    assert_eq!(actual, Some(&NetString::from(expected)), "for {input:?}");
}

/// A lookup the C# test asserts succeeded (`Assert.IsTrue(TryGet…)` or `Assert.NotNull`).
#[track_caller]
pub fn found<T>(value: Option<T>, what: &str, input: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{what} for {input:?}"),
    }
}

/// `GetString(key)`: the text of the value under `key`. `None` if there's no value there, or it's null.
pub fn get_string<'t>(dictionary: DictRef<'t>, key: &str) -> Option<&'t NetString> {
    dictionary.try_get_node(key)?.as_value()?.value()
}

/// `GetDictionary(key)`, which is `TryGetDictionary`: the dictionary under `key`, if that's what is there.
pub fn get_dictionary<'t>(dictionary: DictRef<'t>, key: &str) -> Option<DictRef<'t>> {
    dictionary.try_get_node(key)?.as_dictionary()
}

/// `GetList(key)`, which is `TryGetList`.
pub fn get_list<'t>(dictionary: DictRef<'t>, key: &str) -> Option<ListRef<'t>> {
    dictionary.try_get_node(key)?.as_list()
}

/// `list.GetString(index)`.
pub fn list_get_string<'t>(list: ListRef<'t>, index: i32) -> Option<&'t NetString> {
    list.try_get_node(index)?.as_value()?.value()
}

/// `list.GetDictionary(index)`.
pub fn list_get_dictionary<'t>(list: ListRef<'t>, index: i32) -> Option<DictRef<'t>> {
    list.try_get_node(index)?.as_dictionary()
}
