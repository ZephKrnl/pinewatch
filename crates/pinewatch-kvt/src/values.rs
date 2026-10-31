//! The `IFormattedFileReader` methods of `KeyValueTableReader`, plus the type readers `Level.hierarchy` uses.

use pinewatch_mono::{ExceptionKind, Guid, MonoException, NetString, NumberFormat, NumberStyles, try_parse_i32, try_parse_u32};

use crate::reader::{KvtTable, KvtValue};

/// A handle to one `KeyValueTableReader`. The key and index it reads with live in the table itself, so two
/// handles to the same table share them, like two references to the same C# object.
#[derive(Debug, Clone, Copy)]
pub struct KvtCursor<'a> {
    table: &'a KvtTable,
}

/// `List<object>`'s indexer message for an index out of range.
fn list_index_out_of_range() -> MonoException {
    MonoException::new(
        ExceptionKind::ArgumentOutOfRange,
        "Index was out of range. Must be non-negative and less than the size of the collection.\nParameter name: index",
    )
}

/// `DatValue.INVALID_TYPE_CHARS`.
const INVALID_TYPE_CHARS: [u16; 3] = [b'\\' as u16, b':' as u16, b'/' as u16];

/// `KeyValueTableTypeRedirectorRegistry`'s built-in redirects.
const TYPE_REDIRECTS: [(&str, &str); 2] = [
    (
        "SDG.Framework.Landscapes.PlayerClipVolume, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null",
        "SDG.Framework.Devkit.PlayerClipVolume, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null",
    ),
    (
        "SDG.Framework.Foliage.KillVolume, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null",
        "SDG.Framework.Devkit.KillVolume, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null",
    ),
];

/// `KeyValueTableTypeRedirectorRegistry.chase`.
fn chase(mut name: NetString) -> NetString {
    while let Some((_, new)) = TYPE_REDIRECTS.iter().find(|(old, _)| name == *old) {
        name = NetString::from_str(new);
    }
    name
}

impl<'a> KvtCursor<'a> {
    pub fn new(table: &'a KvtTable) -> Self {
        Self { table }
    }

    pub fn table(&self) -> &'a KvtTable {
        self.table
    }

    /// `containsKey(key)`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.table.contains_key(&NetString::from_str(key))
    }

    /// `readKey(key)`: selects a key and clears the array index (-1).
    pub fn read_key(&mut self, key: &str) {
        let mut state = self.table.state.borrow_mut();
        state.key = Some(NetString::from_str(key));
        state.index = -1;
    }

    /// `readArrayIndex(index)`.
    pub fn read_array_index(&mut self, index: i32) {
        self.table.state.borrow_mut().index = index;
    }

    /// `readArrayLength(key)`.
    pub fn read_array_length(&mut self, key: &str) -> Result<i32, MonoException> {
        self.read_key(key);
        self.read_array_length_current()
    }

    /// `readArrayLength()`: 0 if the key is missing; `NullReferenceException` if its value isn't a list.
    pub fn read_array_length_current(&mut self) -> Result<i32, MonoException> {
        let key = self.current_key()?;
        match self.table.get_net(&key) {
            None => Ok(0),
            Some(KvtValue::List(items)) => Ok(items.len() as i32),
            Some(_) => Err(MonoException::null_reference()),
        }
    }

    fn current_key(&self) -> Result<NetString, MonoException> {
        self.table.state.borrow().key.clone().ok_or_else(|| MonoException::argument_null("key"))
    }

    /// The value selected by the current key and index, as `readValue()` and `readObject()` look it up.
    fn current(&self) -> Result<Option<&'a KvtValue>, MonoException> {
        let key = self.current_key()?;
        let index = self.table.state.borrow().index;
        let Some(value) = self.table.get_net(&key) else { return Ok(None) };
        if index == -1 {
            return Ok(Some(value));
        }
        match value {
            KvtValue::List(items) => usize::try_from(index)
                .ok()
                .and_then(|i| items.get(i))
                .map(Some)
                .ok_or_else(list_index_out_of_range),
            _ => Err(MonoException::null_reference()),
        }
    }

    /// `readValue()`: the selected string, `None` if missing, `InvalidCastException` if it's a table or list.
    pub fn read_value(&mut self) -> Result<Option<NetString>, MonoException> {
        match self.current()? {
            None => Ok(None),
            Some(KvtValue::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(MonoException::invalid_cast()),
        }
    }

    /// `readValue(key)`.
    pub fn read_value_key(&mut self, key: &str) -> Result<Option<NetString>, MonoException> {
        self.read_key(key);
        self.read_value()
    }

    /// `readObject()`: the selected nested reader, or `None` if missing or not a table.
    pub fn read_object(&mut self) -> Result<Option<KvtCursor<'a>>, MonoException> {
        match self.current()? {
            Some(KvtValue::Table(table)) => Ok(Some(KvtCursor::new(table))),
            _ => Ok(None),
        }
    }

    /// `readObject(key)`.
    pub fn read_object_key(&mut self, key: &str) -> Result<Option<KvtCursor<'a>>, MonoException> {
        self.read_key(key);
        self.read_object()
    }

    /// `readObject(index)`.
    pub fn read_object_index(&mut self, index: i32) -> Result<Option<KvtCursor<'a>>, MonoException> {
        self.read_array_index(index);
        self.read_object()
    }

    /// `KeyValueTableIntReader`: `int.TryParse(value, NumberStyles.Any, InvariantCulture)`, 0 on failure.
    pub fn read_i32(&mut self) -> Result<i32, MonoException> {
        let value = self.read_value()?;
        Ok(try_parse_i32(value.as_ref(), NumberStyles::ANY, &NumberFormat::invariant()).unwrap_or(0))
    }

    /// `KeyValueTableUIntReader`.
    pub fn read_u32(&mut self) -> Result<u32, MonoException> {
        let value = self.read_value()?;
        Ok(try_parse_u32(value.as_ref(), NumberStyles::ANY, &NumberFormat::invariant()).unwrap_or(0))
    }

    /// `KeyValueTableGUIDReader`: empty, missing or `"0"` is `Guid.Empty`; otherwise `new Guid(value)`.
    pub fn read_guid(&mut self) -> Result<Guid, MonoException> {
        match self.read_value()? {
            None => Ok(Guid::EMPTY),
            Some(value) if value.is_empty() || value == "0" => Ok(Guid::EMPTY),
            Some(value) => Guid::parse(&value),
        }
    }

    /// `KeyValueTableTypeReader` up to `Type.GetType`: the redirected assembly-qualified name, or `None` if empty
    /// or containing `\`, `:` or `/`. Resolving the name to a type is the caller's job.
    pub fn read_type_name(&mut self) -> Result<Option<NetString>, MonoException> {
        let Some(name) = self.read_value()? else { return Ok(None) };
        if name.is_empty() {
            return Ok(None);
        }
        let name = chase(name);
        if name.as_units().iter().any(|c| INVALID_TYPE_CHARS.contains(c)) {
            return Ok(None);
        }
        Ok(Some(name))
    }
}
