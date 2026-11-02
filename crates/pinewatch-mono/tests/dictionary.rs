//! `Dictionary<TKey, TValue>` as Unity's Mono compiles it (corefx `Dictionary.cs`): iteration in slot order, where a
//! removed key's slot is reused by the next insertion, most recently freed first.

use pinewatch_mono::{ExceptionKind, MonoDictionary};

fn keys(dictionary: &MonoDictionary<u16, &str>) -> Vec<u16> {
    dictionary.iter().map(|(key, _)| *key).collect()
}

#[test]
fn insertion_order_without_removals() {
    let mut dictionary = MonoDictionary::new();
    for key in [5u16, 1, 9, 3] {
        dictionary.add(key, "v").unwrap();
    }
    assert_eq!(keys(&dictionary), [5, 1, 9, 3]);
    assert_eq!(dictionary.len(), 4);
}

/// A removed slot is reused first, so the new key appears where the old one was.
#[test]
fn removed_slots_are_reused_most_recent_first() {
    let mut dictionary = MonoDictionary::new();
    for key in [10u16, 20, 30, 40] {
        dictionary.add(key, "v").unwrap();
    }
    assert_eq!(dictionary.remove(&20), Some("v"));
    assert_eq!(dictionary.remove(&40), Some("v"));
    assert_eq!(keys(&dictionary), [10, 30]);
    dictionary.add(50, "w").unwrap(); // takes 40's slot, freed last
    dictionary.add(60, "w").unwrap(); // takes 20's slot
    dictionary.add(70, "w").unwrap(); // appended
    assert_eq!(keys(&dictionary), [10, 60, 30, 50, 70]);
    assert_eq!(dictionary.len(), 5);
}

#[test]
fn lookups() {
    let mut dictionary = MonoDictionary::new();
    dictionary.add(1u16, "one").unwrap();
    assert!(dictionary.contains_key(&1));
    assert_eq!(dictionary.try_get_value(&1), Some(&"one"));
    assert_eq!(dictionary.try_get_value(&2), None);
    assert_eq!(dictionary.remove(&2), None);
}

/// `Add` with a key that is already there throws `ArgumentException`.
#[test]
fn adding_a_duplicate_key_throws() {
    let mut dictionary = MonoDictionary::new();
    dictionary.add(7u16, "a").unwrap();
    let error = dictionary.add(7, "b").unwrap_err();
    assert_eq!(error.kind, ExceptionKind::Argument);
    assert_eq!(error.message, "An item with the same key has already been added. Key: 7");
    assert_eq!(dictionary.try_get_value(&7), Some(&"a"));
}
