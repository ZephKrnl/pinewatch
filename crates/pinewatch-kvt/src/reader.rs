//! `KeyValueTableReader.readDictionary` and `readList`.

use std::cell::RefCell;
use std::collections::HashMap;

use pinewatch_mono::{MonoException, NetString, decode_stream_reader_default};

const BACKSLASH: u16 = b'\\' as u16;
const QUOTE: u16 = b'"' as u16;
const OPEN_BRACE: u16 = b'{' as u16;
const CLOSE_BRACE: u16 = b'}' as u16;
const OPEN_BRACKET: u16 = b'[' as u16;
const CLOSE_BRACKET: u16 = b']' as u16;

/// A value in the C# `Dictionary<string, object>`: a string, a nested reader, or a `List<object>`.
#[derive(Debug)]
pub enum KvtValue {
    String(NetString),
    Table(KvtTable),
    List(Vec<KvtValue>),
}

/// The reader state that C# keeps on each `KeyValueTableReader` instance. A new reader starts with
/// `key = null` and `index = 0`.
#[derive(Debug, Clone, Default)]
pub(crate) struct CursorState {
    pub key: Option<NetString>,
    pub index: i32,
}

/// One `KeyValueTableReader`: its table (insertion-ordered, exact-match keys) and its own cursor state.
#[derive(Debug, Default)]
pub struct KvtTable {
    entries: Vec<(NetString, KvtValue)>,
    positions: HashMap<NetString, usize>,
    pub(crate) state: RefCell<CursorState>,
}

impl KvtTable {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&KvtValue> {
        self.get_net(&NetString::from_str(key))
    }

    pub fn get_net(&self, key: &NetString) -> Option<&KvtValue> {
        self.positions.get(key).map(|&i| &self.entries[i].1)
    }

    pub fn contains_key(&self, key: &NetString) -> bool {
        self.positions.contains_key(key)
    }

    /// Keys in insertion order (the C# dictionary never removes entries here, so this is its enumeration order).
    pub fn keys(&self) -> impl Iterator<Item = &NetString> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NetString, &KvtValue)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    fn get_mut_net(&mut self, key: &NetString) -> Option<&mut KvtValue> {
        let i = *self.positions.get(key)?;
        Some(&mut self.entries[i].1)
    }

    /// `Dictionary.Add` for a key the caller has checked is absent.
    fn add(&mut self, key: NetString, value: KvtValue) {
        self.positions.insert(key.clone(), self.entries.len());
        self.entries.push((key, value));
    }
}

/// The `StreamReader` the C# reader pulls characters from.
struct Input<'a> {
    units: &'a [u16],
    pos: usize,
}

impl Input<'_> {
    fn end_of_stream(&self) -> bool {
        self.pos >= self.units.len()
    }

    fn read(&mut self) -> u16 {
        let c = self.units[self.pos];
        self.pos += 1;
        c
    }
}

/// `new KeyValueTableReader(streamReader)` over already-decoded text.
pub fn read_kvt(text: &NetString) -> Result<KvtTable, MonoException> {
    let mut input = Input { units: text.as_units(), pos: 0 };
    // C#'s builder is a static StringBuilder shared by every nested reader. Its leftover content never reaches
    // a result (each string starts by clearing it), but sharing it keeps the port identical.
    let mut builder = Vec::new();
    let mut table = KvtTable::default();
    read_dictionary(&mut input, &mut table, &mut builder)?;
    Ok(table)
}

/// Decodes a file the way `new StreamReader(stream)` does, then reads it.
pub fn read_kvt_bytes(bytes: &[u8]) -> Result<KvtTable, MonoException> {
    read_kvt(&decode_stream_reader_default(bytes))
}

/// `KeyValueTableReader.readDictionary`.
fn read_dictionary(input: &mut Input, scope: &mut KvtTable, builder: &mut Vec<u16>) -> Result<(), MonoException> {
    let mut dictionary_key: Option<NetString> = None;
    let mut in_quotes = false;
    let mut ignore_next_char = false;

    while !input.end_of_stream() {
        let character = input.read();

        if ignore_next_char {
            builder.push(character);
            ignore_next_char = false;
            continue;
        }
        if character == BACKSLASH {
            ignore_next_char = true;
            continue;
        }

        if character == QUOTE {
            if in_quotes {
                in_quotes = false;
                if dictionary_key.as_ref().is_none_or(NetString::is_empty) {
                    dictionary_key = Some(NetString::from_units(builder.clone()));
                } else if let Some(key) = dictionary_key.take() {
                    let value = NetString::from_units(builder.clone());
                    if !scope.contains_key(&key) {
                        scope.add(key, KvtValue::String(value));
                    }
                }
            } else {
                in_quotes = true;
                builder.clear();
            }
        } else if in_quotes {
            builder.push(character);
        } else if character == OPEN_BRACE {
            // scope.TryGetValue(dictionaryKey, ...) throws for a null key.
            let key = dictionary_key.take().ok_or_else(|| MonoException::argument_null("key"))?;
            match scope.get_mut_net(&key) {
                Some(KvtValue::Table(existing)) => read_dictionary(input, existing, builder)?,
                Some(_) => return Err(MonoException::invalid_cast()),
                None => {
                    let mut nested = KvtTable::default();
                    read_dictionary(input, &mut nested, builder)?;
                    scope.add(key, KvtValue::Table(nested));
                }
            }
        } else if character == CLOSE_BRACE {
            return Ok(());
        } else if character == OPEN_BRACKET {
            let key = dictionary_key.take().ok_or_else(|| MonoException::argument_null("key"))?;
            if !scope.contains_key(&key) {
                scope.add(key.clone(), KvtValue::List(Vec::new()));
            }
            match scope.get_mut_net(&key) {
                Some(KvtValue::List(list)) => read_list(input, list, builder)?,
                _ => return Err(MonoException::invalid_cast()),
            }
        }
    }
    Ok(())
}

/// `KeyValueTableReader.readList`. Nested lists are not supported: `[` is ignored and `]` ends this list.
fn read_list(input: &mut Input, scope: &mut Vec<KvtValue>, builder: &mut Vec<u16>) -> Result<(), MonoException> {
    let mut in_quotes = false;
    let mut ignore_next_char = false;

    while !input.end_of_stream() {
        let character = input.read();

        if ignore_next_char {
            builder.push(character);
            ignore_next_char = false;
            continue;
        }
        if character == BACKSLASH {
            ignore_next_char = true;
            continue;
        }

        if character == QUOTE {
            if in_quotes {
                in_quotes = false;
                scope.push(KvtValue::String(NetString::from_units(builder.clone())));
            } else {
                in_quotes = true;
                builder.clear();
            }
        } else if in_quotes {
            builder.push(character);
        } else if character == OPEN_BRACE {
            let mut nested = KvtTable::default();
            read_dictionary(input, &mut nested, builder)?;
            scope.push(KvtValue::Table(nested));
        } else if character == CLOSE_BRACKET {
            return Ok(());
        }
    }
    Ok(())
}
