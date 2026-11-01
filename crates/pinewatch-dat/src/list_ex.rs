//! `DatListEx` (UnturnedDat/DatListEx.cs): reading list items by index.

use pinewatch_mono::NetString;

use crate::tree::{DictRef, ListRef, ValueRef};

impl<'t> ListRef<'t> {
    /// `TryGetValue(index, out IDatValue value)`: the item at `index`, if it's a value.
    pub fn try_get_value(&self, index: i32) -> Option<ValueRef<'t>> {
        self.try_get_node(index)?.as_value()
    }

    /// `TryGetDictionary(index, out dictionary)`, which is also `GetDictionary(index)`.
    pub fn try_get_dictionary(&self, index: i32) -> Option<DictRef<'t>> {
        self.try_get_node(index)?.as_dictionary()
    }

    /// `TryGetList(index, out list)`, which is also `GetList(index)`.
    pub fn try_get_list(&self, index: i32) -> Option<ListRef<'t>> {
        self.try_get_node(index)?.as_list()
    }

    /// `TryGetString(index, out value)`: `Some` when the item is a value, with its text, which is `None` where the
    /// C# has null.
    pub fn try_get_string(&self, index: i32) -> Option<Option<&'t NetString>> {
        Some(self.try_get_value(index)?.value())
    }

    /// `GetString(index, defaultValue)`: the text when the item is a value, even when that text is null, and
    /// otherwise `default`.
    pub fn get_string<'a>(&self, index: i32, default: Option<&'a NetString>) -> Option<&'a NetString>
    where
        't: 'a,
    {
        match self.try_get_string(index) {
            Some(value) => value,
            None => default,
        }
    }
}
