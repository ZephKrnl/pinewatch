//! `IDatParseable` (UnturnedDat/DatParseable.cs) and the struct helpers of `DatNodeEx`, `DatDictionaryEx` and
//! `DatListEx`.
//!
//! The C# starts from `default(T)` and lets `TryParse` fill it in, so a parse that fails halfway hands back a partly
//! filled value. `T::default()` stands for `default(T)`, so an implementation's `Default` must set every field to
//! zero, as C# does. An exception thrown inside `TryParse` leaves the helpers, as it leaves the C#: `Err`.

use pinewatch_mono::MonoException;

use crate::tree::{DictRef, ListRef, NodeRef};

/// `IDatParseable`: a struct that reads itself from a node.
pub trait DatParseable {
    /// What `TryParse` needs besides the node: the game state the C# reads from statics, for example
    /// `Assets.currentMasterBundle`, or where `Assets.ReportError` reports. `()` when there's none.
    type Context: ?Sized;

    /// `TryParse(node)`: reads the node into `self` and says whether that worked. It may change `self` even when
    /// it fails. `Err` is an exception thrown inside it.
    fn try_parse(&mut self, node: NodeRef<'_>, context: &mut Self::Context) -> Result<bool, MonoException>;
}

impl NodeRef<'_> {
    /// `TryParseStruct<T>(out value)`: the result and the value, which can be partly filled when the result is false.
    pub fn try_parse_struct<T: DatParseable + Default>(
        &self,
        context: &mut T::Context,
    ) -> Result<(bool, T), MonoException> {
        let mut value = T::default();
        let success = value.try_parse(*self, context)?;
        Ok((success, value))
    }

    /// `ParseStruct<T>(defaultValue)`: the parsed value, or `default` when parsing fails.
    pub fn parse_struct<T: DatParseable + Default>(
        &self,
        default: T,
        context: &mut T::Context,
    ) -> Result<T, MonoException> {
        match self.try_parse_struct(context)? {
            (true, value) => Ok(value),
            (false, _) => Ok(default),
        }
    }
}

impl DictRef<'_> {
    /// `TryParseStruct<T>(key, out value)`: `NodeRef::try_parse_struct` on any node under `key`. A missing key gives
    /// false and `T::default()`.
    pub fn try_parse_struct<T: DatParseable + Default>(
        &self,
        key: &str,
        context: &mut T::Context,
    ) -> Result<(bool, T), MonoException> {
        let mut value = T::default();
        let success = match self.try_get_node(key) {
            Some(node) => value.try_parse(node, context)?,
            None => false,
        };
        Ok((success, value))
    }

    /// `ParseStruct<T>(key, defaultValue)`.
    pub fn parse_struct<T: DatParseable + Default>(
        &self,
        key: &str,
        default: T,
        context: &mut T::Context,
    ) -> Result<T, MonoException> {
        match self.try_parse_struct(key, context)? {
            (true, value) => Ok(value),
            (false, _) => Ok(default),
        }
    }

    /// `ParseListOfStructs<T>(key)`: `None` (null) unless the key holds a list.
    pub fn parse_list_of_structs<T: DatParseable + Default>(
        &self,
        key: &str,
        context: &mut T::Context,
    ) -> Result<Option<Vec<T>>, MonoException> {
        match self.try_get_list(key) {
            Some(list) => Ok(Some(list.parse_list_of_structs(context)?)),
            None => Ok(None),
        }
    }

    /// `ParseArrayOfStructs<T>(key, defaultValue)`: `None` (null) unless the key holds a list.
    pub fn parse_array_of_structs<T: DatParseable + Default + Clone>(
        &self,
        key: &str,
        default: T,
        context: &mut T::Context,
    ) -> Result<Option<Vec<T>>, MonoException> {
        match self.try_get_list(key) {
            Some(list) => Ok(Some(list.parse_array_of_structs(default, context)?)),
            None => Ok(None),
        }
    }
}

impl ListRef<'_> {
    /// `ParseListOfStructs<T>()`: the items that parse, in order.
    pub fn parse_list_of_structs<T: DatParseable + Default>(
        &self,
        context: &mut T::Context,
    ) -> Result<Vec<T>, MonoException> {
        let mut results = Vec::with_capacity(self.len());
        for node in self.iter() {
            if let (true, value) = node.try_parse_struct(context)? {
                results.push(value);
            }
        }
        Ok(results)
    }

    /// `ParseArrayOfStructs<T>(defaultValue)`: one entry per item, `default` for each item that doesn't parse.
    pub fn parse_array_of_structs<T: DatParseable + Default + Clone>(
        &self,
        default: T,
        context: &mut T::Context,
    ) -> Result<Vec<T>, MonoException> {
        let mut results = Vec::with_capacity(self.len());
        for node in self.iter() {
            match node.try_parse_struct(context)? {
                (true, value) => results.push(value),
                (false, _) => results.push(default.clone()),
            }
        }
        Ok(results)
    }
}
