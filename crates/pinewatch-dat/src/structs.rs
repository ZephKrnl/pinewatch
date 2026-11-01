//! `IDatParseable` (UnturnedDat/DatParseable.cs) and the struct helpers of `DatNodeEx`, `DatDictionaryEx` and
//! `DatListEx`.
//!
//! The C# starts from `default(T)` and lets `TryParse` fill it in, so a parse that fails halfway hands back a partly
//! filled value. `T::default()` stands for `default(T)`, so an implementation's `Default` must set every field to
//! zero, as C# does.

use crate::tree::{DictRef, ListRef, NodeRef};

/// `IDatParseable`: a struct that reads itself from a node.
pub trait DatParseable {
    /// `TryParse(node)`: reads the node into `self` and says whether that worked. It may change `self` even when
    /// it fails.
    fn try_parse(&mut self, node: NodeRef<'_>) -> bool;
}

impl NodeRef<'_> {
    /// `TryParseStruct<T>(out value)`: the result and the value, which can be partly filled when the result is false.
    pub fn try_parse_struct<T: DatParseable + Default>(&self) -> (bool, T) {
        let mut value = T::default();
        let success = value.try_parse(*self);
        (success, value)
    }

    /// `ParseStruct<T>(defaultValue)`: the parsed value, or `default` when parsing fails.
    pub fn parse_struct<T: DatParseable + Default>(&self, default: T) -> T {
        match self.try_parse_struct() {
            (true, value) => value,
            (false, _) => default,
        }
    }
}

impl DictRef<'_> {
    /// `TryParseStruct<T>(key, out value)`: `NodeRef::try_parse_struct` on any node under `key`. A missing key gives
    /// false and `T::default()`.
    pub fn try_parse_struct<T: DatParseable + Default>(&self, key: &str) -> (bool, T) {
        let mut value = T::default();
        let success = match self.try_get_node(key) {
            Some(node) => value.try_parse(node),
            None => false,
        };
        (success, value)
    }

    /// `ParseStruct<T>(key, defaultValue)`.
    pub fn parse_struct<T: DatParseable + Default>(&self, key: &str, default: T) -> T {
        match self.try_parse_struct(key) {
            (true, value) => value,
            (false, _) => default,
        }
    }

    /// `ParseListOfStructs<T>(key)`: `None` (null) unless the key holds a list.
    pub fn parse_list_of_structs<T: DatParseable + Default>(&self, key: &str) -> Option<Vec<T>> {
        Some(self.try_get_list(key)?.parse_list_of_structs())
    }

    /// `ParseArrayOfStructs<T>(key, defaultValue)`: `None` (null) unless the key holds a list.
    pub fn parse_array_of_structs<T: DatParseable + Default + Clone>(&self, key: &str, default: T) -> Option<Vec<T>> {
        Some(self.try_get_list(key)?.parse_array_of_structs(default))
    }
}

impl ListRef<'_> {
    /// `ParseListOfStructs<T>()`: the items that parse, in order.
    pub fn parse_list_of_structs<T: DatParseable + Default>(&self) -> Vec<T> {
        let mut results = Vec::with_capacity(self.len());
        for node in self.iter() {
            if let (true, value) = node.try_parse_struct() {
                results.push(value);
            }
        }
        results
    }

    /// `ParseArrayOfStructs<T>(defaultValue)`: one entry per item, `default` for each item that doesn't parse.
    pub fn parse_array_of_structs<T: DatParseable + Default + Clone>(&self, default: T) -> Vec<T> {
        let mut results = Vec::with_capacity(self.len());
        for node in self.iter() {
            match node.try_parse_struct() {
                (true, value) => results.push(value),
                (false, _) => results.push(default.clone()),
            }
        }
        results
    }
}
