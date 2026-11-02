// Ported from corefx `src/Common/src/CoreLib/System/Collections/Generic/Dictionary.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! `Dictionary<TKey, TValue>`'s observable order: entries live in slots, iteration goes through the slots in order,
//! a removed key frees its slot, and the next insertion takes the most recently freed slot (the free list is
//! last-in, first-out) before appending a new one. Lookups go through a Rust map; only the order is ported.

use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;

use crate::{MonoException, sr};

/// A `Dictionary<TKey, TValue>` with the default comparer.
#[derive(Debug, Clone)]
pub struct MonoDictionary<K, V> {
    slots: Vec<Option<(K, V)>>,
    /// `_freeList`, newest first at the end.
    free: Vec<usize>,
    index: HashMap<K, usize>,
}

impl<K, V> Default for MonoDictionary<K, V> {
    fn default() -> Self {
        Self { slots: Vec::new(), free: Vec::new(), index: HashMap::new() }
    }
}

impl<K: Eq + Hash + Clone + Display, V> MonoDictionary<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Count`.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// `ContainsKey`.
    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// `TryGetValue`.
    pub fn try_get_value(&self, key: &K) -> Option<&V> {
        let slot = *self.index.get(key)?;
        self.slots[slot].as_ref().map(|(_, value)| value)
    }

    /// `Add`: throws `ArgumentException` for a key that is already there.
    pub fn add(&mut self, key: K, value: V) -> Result<(), MonoException> {
        if self.index.contains_key(&key) {
            let message = sr::ARGUMENT_ADDING_DUPLICATE_WITH_KEY.replace("{0}", &key.to_string());
            return Err(MonoException::argument(&message, None));
        }
        let slot = match self.free.pop() {
            Some(slot) => slot,
            None => {
                self.slots.push(None);
                self.slots.len() - 1
            }
        };
        self.slots[slot] = Some((key.clone(), value));
        self.index.insert(key, slot);
        Ok(())
    }

    /// `Remove`: the value, if the key was there. Its slot goes on the free list.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let slot = self.index.remove(key)?;
        self.free.push(slot);
        self.slots[slot].take().map(|(_, value)| value)
    }

    /// The entries in the dictionary's enumeration order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().filter_map(|slot| slot.as_ref().map(|(key, value)| (key, value)))
    }
}
