//! Helpers shared by the parser tests. The lookups are the C# tests' `DatDictionaryEx` and `DatListEx` calls,
//! written on the tree API with the same results.

#![allow(dead_code)]

use std::collections::HashSet;

use pinewatch_dat::{DatParser, DatTree, DictRef, ListRef, ValueRef};
use pinewatch_mono::{EnumInfo, EnumUnderlying, NetEnum, NetString};

fn u16_at(d: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([d[at], d[at + 1]]))
}

fn u32_at(d: &[u8], at: usize) -> usize {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]) as usize
}

/// The string literals of a .NET assembly: the entries of its `#US` metadata heap (ECMA-335 II.24.2.4).
pub fn user_strings(d: &[u8]) -> HashSet<String> {
    let pe = u32_at(d, 0x3C);
    let section_count = u16_at(d, pe + 6);
    let optional_size = u16_at(d, pe + 20);
    let optional = pe + 24;
    let directories = optional + if u16_at(d, optional) == 0x10B { 96 } else { 112 };
    let section_table = optional + optional_size;
    let offset = |rva: usize| -> usize {
        for i in 0..section_count {
            let s = section_table + i * 40;
            let (virtual_size, va, raw_size, raw) =
                (u32_at(d, s + 8), u32_at(d, s + 12), u32_at(d, s + 16), u32_at(d, s + 20));
            if va <= rva && rva < va + virtual_size.max(raw_size) {
                return rva - va + raw;
            }
        }
        panic!("RVA {rva:#x} is outside every section");
    };
    let cli = offset(u32_at(d, directories + 14 * 8));
    let metadata = offset(u32_at(d, cli + 8));
    let mut p = metadata + 16 + u32_at(d, metadata + 12);
    let stream_count = u16_at(d, p + 2);
    p += 4;
    for _ in 0..stream_count {
        let (start, size) = (u32_at(d, p), u32_at(d, p + 4));
        p += 8;
        let name_end = p + d[p..].iter().position(|&b| b == 0).expect("stream name");
        let name = &d[p..name_end];
        p = (name_end + 4) & !3;
        if name == b"#US" {
            return heap_entries(&d[metadata + start..metadata + start + size]);
        }
    }
    panic!("no #US heap");
}

fn heap_entries(heap: &[u8]) -> HashSet<String> {
    let mut strings = HashSet::new();
    let mut i = 1; // Entry 0 is the empty string.
    while i < heap.len() {
        let b = usize::from(heap[i]);
        let (length, header) = if b & 0x80 == 0 {
            (b, 1)
        } else if b & 0xC0 == 0x80 {
            (((b & 0x3F) << 8) | usize::from(heap[i + 1]), 2)
        } else {
            let rest = (usize::from(heap[i + 1]) << 16) | (usize::from(heap[i + 2]) << 8) | usize::from(heap[i + 3]);
            (((b & 0x1F) << 24) | rest, 4)
        };
        let start = i + header;
        if length > 0 {
            // UTF-16 code units, then one flag byte.
            let units: Vec<u16> =
                heap[start..start + length - 1].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
            strings.insert(String::from_utf16_lossy(&units));
        }
        i = start + length;
    }
    strings
}

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
