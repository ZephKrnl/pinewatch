//! `DatDictionaryEx` (UnturnedDat/DatDictionaryEx.cs): looking a key up and reading its value.
//!
//! Each `try_parse_x(key)` is `TryParseX(key, out value)`: `None` unless the key holds a value node that parses.
//! On failure the C# out value is always the type's default, which is `unwrap_or_default()`.

use pinewatch_mono::{Guid, MonoException, NetEnum, NetString};

use crate::tree::{DictRef, ListRef, ValueRef};
use crate::types::{TypeHandle, TypeResolver};

/// A `TryParseX(key)`/`ParseX(key)` pair that reads the value under `key` as `ValueRef` does.
macro_rules! keyed_pair {
    ($try_name:ident, $name:ident, $ty:ty) => {
        #[doc = concat!("`ValueRef::", stringify!($try_name), "` on the value under `key`.")]
        pub fn $try_name(&self, key: &str) -> Option<$ty> {
            self.try_get_value(key)?.$try_name()
        }

        #[doc = concat!("The `", stringify!($try_name), "` result, or `default`.")]
        pub fn $name(&self, key: &str, default: $ty) -> $ty {
            self.$try_name(key).unwrap_or(default)
        }
    };
}

impl<'t> DictRef<'t> {
    /// `TryGetValue(key, out IDatValue node)`: the node under `key`, if it's a value.
    pub fn try_get_value(&self, key: &str) -> Option<ValueRef<'t>> {
        self.try_get_node(key)?.as_value()
    }

    /// `TryGetDictionary(key, out node)`, which is also `GetDictionary(key)`.
    pub fn try_get_dictionary(&self, key: &str) -> Option<DictRef<'t>> {
        self.try_get_node(key)?.as_dictionary()
    }

    /// `TryGetList(key, out node)`, which is also `GetList(key)`.
    pub fn try_get_list(&self, key: &str) -> Option<ListRef<'t>> {
        self.try_get_node(key)?.as_list()
    }

    /// `TryGetString(key, out value)`: `Some` when the key holds a value, with its text. The text is `None` where
    /// the C# has null, as for a flag, and the C# still returns true then.
    pub fn try_get_string(&self, key: &str) -> Option<Option<&'t NetString>> {
        Some(self.try_get_value(key)?.value())
    }

    /// `GetString(key, defaultValue)`: the text when the key holds a value, even when that text is null, and
    /// otherwise `default`.
    pub fn get_string<'a>(&self, key: &str, default: Option<&'a NetString>) -> Option<&'a NetString>
    where
        't: 'a,
    {
        match self.try_get_string(key) {
            Some(value) => value,
            None => default,
        }
    }

    keyed_pair!(try_parse_int8, parse_int8, i8);
    keyed_pair!(try_parse_uint8, parse_uint8, u8);
    keyed_pair!(try_parse_int16, parse_int16, i16);
    keyed_pair!(try_parse_uint16, parse_uint16, u16);
    keyed_pair!(try_parse_int32, parse_int32, i32);
    keyed_pair!(try_parse_uint32, parse_uint32, u32);
    keyed_pair!(try_parse_int64, parse_int64, i64);
    keyed_pair!(try_parse_uint64, parse_uint64, u64);
    keyed_pair!(try_parse_float, parse_float, f32);
    keyed_pair!(try_parse_double, parse_double, f64);
    keyed_pair!(try_parse_bool, parse_bool, bool);
    keyed_pair!(try_parse_guid, parse_guid, Guid);

    /// `ValueRef::try_parse_enum` on the value under `key`.
    pub fn try_parse_enum<T: NetEnum>(&self, key: &str) -> Option<T> {
        self.try_get_value(key)?.try_parse_enum()
    }

    /// The `try_parse_enum` result, or `default`.
    pub fn parse_enum<T: NetEnum>(&self, key: &str, default: T) -> T {
        self.try_parse_enum(key).unwrap_or(default)
    }

    /// `ParseType(key, defaultValue)`: `ValueRef::parse_type` on the value under `key`, or `default`.
    pub fn parse_type(
        &self,
        key: &str,
        resolver: &dyn TypeResolver,
        default: Option<TypeHandle>,
    ) -> Result<Option<TypeHandle>, MonoException> {
        match self.try_get_value(key) {
            Some(node) => node.parse_type(resolver, default),
            None => Ok(default),
        }
    }
}
