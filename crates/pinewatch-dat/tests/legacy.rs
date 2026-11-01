//! The legacy line reader, `Data` (`Assembly-CSharp/Unturned/Files/Data.cs`): one `key value` per line.

mod common;

use common::DatTestEnum;
use pinewatch_dat::LegacyData;
use pinewatch_mono::{Guid, NetString};
use pinewatch_unity::{Color, Color32, Vector3};

fn data(content: &str) -> LegacyData {
    LegacyData::new(&NetString::from(content))
}

fn text(value: &str) -> NetString {
    NetString::from(value)
}

#[test]
fn key_is_split_at_the_first_space() {
    let data = data("Name Big Rock 2\nFlag");
    assert_eq!(Some(&text("Big Rock 2")), data.try_read_string("Name"));
    assert_eq!(Some(&text("")), data.try_read_string("Flag"));
    assert!(data.errors().is_none());
}

#[test]
fn keys_are_case_sensitive() {
    let data = data("Name x");
    assert!(data.has("Name"));
    assert!(!data.has("name"));
    let default = text("default");
    assert_eq!(Some(&default), data.read_string("name", Some(&default)));
    assert_eq!(None, data.read_string("name", None));
}

/// Only a line starting with `//` is a comment. A line starting with a space has an empty key.
#[test]
fn comments_only_at_line_start() {
    let data = data("// x\nKey value // x\n //y\n/");
    assert_eq!(Some(&text("value // x")), data.try_read_string("Key"));
    assert_eq!(Some(&text("//y")), data.try_read_string(""));
    assert_eq!(Some(&text("")), data.try_read_string("/"));
    assert!(!data.has("//"));
    assert_eq!(3, data.get_contents().len());
}

#[test]
fn duplicate_key_last_wins_with_error() {
    let data = data("A 1\nB 2\nA 3");
    assert_eq!(Some(&[text("Duplicate key: \"A\" Old value: 1 New value: 3")][..]), data.errors());
    assert_eq!(Some(&text("3")), data.try_read_string("A"));
    let keys: Vec<&NetString> = data.get_contents().iter().map(|(key, _)| key).collect();
    assert_eq!(keys, [&text("A"), &text("B")]);
}

/// `readBoolean`: `y` or `true` in any case, or exactly `1`. Anything else is false; a missing key is the default.
#[test]
fn bool_forms() {
    for (value, expected) in [
        ("y", true),
        ("Y", true),
        ("1", true),
        ("true", true),
        ("TRUE", true),
        ("True", true),
        ("yes", false),
        ("t", false),
        ("0", false),
        ("false", false),
        (" true", false),
        ("", false),
    ] {
        let data = data(&format!("Key {value}"));
        assert_eq!(expected, data.read_boolean("Key", !expected), "for {value:?}");
    }
    assert!(data("").read_boolean("Key", true));
}

/// Lines end at `\r`, `\n` or `\r\n`, as `StringReader.ReadLine` splits them, and the last line needs no end.
#[test]
fn line_breaks() {
    let data = data("A 1\rB 2\nC 3\r\nD 4\n\r\nE 5");
    for (key, value) in [("A", "1"), ("B", "2"), ("C", "3"), ("D", "4"), ("E", "5")] {
        assert_eq!(Some(&text(value)), data.try_read_string(key), "{key}");
    }
    assert_eq!(5, data.get_contents().len());
    assert!(data.errors().is_none());
    assert!(LegacyData::new(&NetString::new()).is_empty());
}

/// Numbers parse with `NumberStyles.Any` and the invariant culture, and fall back to the default.
#[test]
fn numbers() {
    let data = data("Byte 255\nBig 1,000\nNegative (5)\nFloat 1.5\nBad x\nHuge 300");
    assert_eq!(255, data.read_byte("Byte", 7));
    assert_eq!(7, data.read_byte("Huge", 7));
    assert_eq!(1000, data.read_int32("Big", 0));
    assert_eq!(-5, data.read_int16("Negative", 0));
    assert_eq!(-5, data.read_sbyte("Negative", 0));
    assert_eq!(1.5, data.read_single("Float", 0.0));
    assert_eq!(9, data.read_uint16("Bad", 9));
    assert_eq!(9, data.read_uint32("Missing", 9));
    assert_eq!(1000, data.read_int64("Big", 0));
    assert_eq!(300, data.read_uint64("Huge", 0));
    assert_eq!(300, data.read_steam_id("Huge"));
    assert_eq!(0, data.read_steam_id("Bad"));
}

/// `readEnum` ignores case and falls back to the default.
#[test]
fn enums() {
    let data = data("Animal bear\nOther cat");
    assert_eq!(DatTestEnum::BEAR, data.read_enum("Animal", DatTestEnum::DOG));
    assert_eq!(DatTestEnum::DOG, data.read_enum("Other", DatTestEnum::DOG));
    assert_eq!(DatTestEnum::DOG, data.read_enum("Missing", DatTestEnum::DOG));
}

/// Vectors and colors come from separate `_X`/`_Y`/`_Z` and `_R`/`_G`/`_B` keys. `readQuaternion` passes the
/// doubled `_X` byte and the `_Y` and `_Z` bytes to `Quaternion.Euler`; the reader returns those angles (P6).
#[test]
fn vectors_and_colors() {
    let data = data("Pos_X 1\nPos_Y 2.5\nRot_X 100\nRot_Y 7\nRot_Z 300\nTint_R 0.5\nTint_B 0.25\nByte_G 9");
    assert_eq!(Vector3::new(1.0, 2.5, 0.0), data.read_vector3("Pos"));
    assert_eq!(Vector3::new(200.0, 7.0, 0.0), data.read_quaternion_euler_angles("Rot"));
    assert_eq!(Color::rgb(0.5, 0.0, 0.25), data.read_color("Tint"));
    assert_eq!(Color::rgb(0.5, 0.75, 0.25), data.read_color_with_default("Tint", Color::new(0.1, 0.75, 0.2, 0.0)));
    let default = Color32::new(1, 2, 3, 4);
    assert_eq!(Color32::new(1, 9, 3, u8::MAX), data.read_color32_rgb("Byte", default));
}

/// `readGUID`: empty, missing and `0` are `Guid.Empty`; anything else goes to `new Guid(value)`, which throws for
/// text that isn't a GUID.
#[test]
fn guids() {
    let guid = "4d362f61e4324509844feedd1a69ae19";
    let data = data(&format!("Good {guid}\nZero 0\nEmpty\nBad xyz"));
    assert_eq!(Ok(Guid::parse(&text(guid)).expect("valid")), data.read_guid("Good"));
    for key in ["Zero", "Empty", "Missing"] {
        assert_eq!(Ok(Guid::default()), data.read_guid(key), "{key}");
    }
    assert_eq!(Err(Guid::parse(&text("xyz")).expect_err("invalid")), data.read_guid("Bad"));
}

/// `ReadGuidOrLegacyId`: a `ushort` first, then a GUID, else neither.
#[test]
fn guid_or_legacy_id() {
    let guid = "4d362f61e4324509844feedd1a69ae19";
    let data = data(&format!("Id 342\nGuid {guid}\nBig 70000\nZero 0\nBad xyz"));
    assert_eq!((Guid::default(), 342), data.read_guid_or_legacy_id("Id"));
    assert_eq!((Guid::parse(&text(guid)).expect("valid"), 0), data.read_guid_or_legacy_id("Guid"));
    for key in ["Big", "Zero", "Bad", "Missing"] {
        assert_eq!((Guid::default(), 0), data.read_guid_or_legacy_id(key), "{key}");
    }
}

/// `getContents`, `getValuesWithKey` and `getKeysWithValue` go through the pairs in insertion order and compare
/// exactly.
#[test]
fn getters() {
    let data = data("A same\nB other\nC same");
    let contents: Vec<(&NetString, &NetString)> = data.get_contents().iter().map(|(key, value)| (key, value)).collect();
    assert_eq!(contents, [(&text("A"), &text("same")), (&text("B"), &text("other")), (&text("C"), &text("same"))]);
    assert_eq!(data.get_values_with_key("B"), [&text("other")]);
    assert!(data.get_values_with_key("b").is_empty());
    assert_eq!(data.get_keys_with_value("same"), [&text("A"), &text("C")]);
    assert!(data.get_keys_with_value("Same").is_empty());
    assert!(!data.is_empty());
}
