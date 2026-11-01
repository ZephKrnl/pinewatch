//! Behavior of `KeyValueTableReader` (Framework/IO/FormattedFiles/KeyValueTables/KeyValueTableReader.cs)
//! and the type readers `Level.hierarchy` uses.

use pinewatch_kvt::{KvtCursor, KvtTable, KvtValue, read_kvt};
use pinewatch_mono::{ExceptionKind, Guid, MonoException, NetString, sr};

fn read(text: &str) -> KvtTable {
    read_kvt(&NetString::from_str(text)).expect("parses")
}

fn read_err(text: &str) -> ExceptionKind {
    read_kvt(&NetString::from_str(text)).expect_err("throws").kind
}

fn s(value: &KvtValue) -> String {
    match value {
        KvtValue::String(v) => v.to_string_lossy(),
        other => panic!("expected a string, got {other:?}"),
    }
}

fn keys(table: &KvtTable) -> Vec<String> {
    table.keys().map(NetString::to_string_lossy).collect()
}

#[test]
fn quoted_pairs_in_order() {
    let t = read("\"A\" \"1\"\n\"B\" \"2\"");
    assert_eq!(keys(&t), ["A", "B"]);
    assert_eq!(s(t.get("A").unwrap()), "1");
    assert_eq!(s(t.get("B").unwrap()), "2");
}

#[test]
fn backslash_makes_next_character_literal() {
    let t = read("\"A\" \"x\\\"y\" \"B\" \"x\\ny\"");
    assert_eq!(s(t.get("A").unwrap()), "x\"y");
    assert_eq!(s(t.get("B").unwrap()), "xny");
}

#[test]
fn escaped_quote_outside_quotes_does_not_start_a_string() {
    // '\"' outside quotes goes into the shared builder, 'K' is ignored, then `" "` becomes a key with no value.
    let t = read("\\\"K\" \"V\"");
    assert_eq!(t.len(), 0);
}

#[test]
fn nested_table() {
    let t = read("\"T\" { \"X\" \"1\" }");
    let KvtValue::Table(inner) = t.get("T").unwrap() else { panic!() };
    assert_eq!(s(inner.get("X").unwrap()), "1");
}

#[test]
fn list_of_strings_and_tables() {
    let t = read("\"L\" [ \"a\" { \"X\" \"1\" } \"b\" ]");
    let KvtValue::List(items) = t.get("L").unwrap() else { panic!() };
    assert_eq!(items.len(), 3);
    assert_eq!(s(&items[0]), "a");
    let KvtValue::Table(inner) = &items[1] else { panic!() };
    assert_eq!(s(inner.get("X").unwrap()), "1");
    assert_eq!(s(&items[2]), "b");
}

#[test]
fn duplicate_scalar_key_keeps_first_value() {
    let t = read("\"A\" \"1\" \"A\" \"2\"");
    assert_eq!(s(t.get("A").unwrap()), "1");
    assert_eq!(t.len(), 1);
}

#[test]
fn repeated_table_key_merges() {
    let t = read("\"T\" { \"X\" \"1\" } \"T\" { \"Y\" \"2\" \"X\" \"3\" }");
    let KvtValue::Table(inner) = t.get("T").unwrap() else { panic!() };
    assert_eq!(keys(inner), ["X", "Y"]);
    assert_eq!(s(inner.get("X").unwrap()), "1");
}

#[test]
fn repeated_list_key_appends() {
    let t = read("\"L\" [ \"a\" ] \"L\" [ \"b\" ]");
    let KvtValue::List(items) = t.get("L").unwrap() else { panic!() };
    assert_eq!(items.iter().map(s).collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn inner_bracket_is_ignored_and_its_close_ends_the_outer_list() {
    let t = read("\"L\" [ \"a\" [ \"b\" ] \"c\" ] \"Z\" \"1\"");
    let KvtValue::List(items) = t.get("L").unwrap() else { panic!() };
    assert_eq!(items.iter().map(s).collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(s(t.get("c").unwrap()), "Z");
    assert!(t.get("Z").is_none());
}

#[test]
fn stray_root_close_brace_ends_parsing() {
    let t = read("\"A\" \"1\" } \"B\" \"2\"");
    assert_eq!(keys(&t), ["A"]);
}

#[test]
fn text_outside_quotes_is_ignored() {
    let t = read("A \"K\" junk \"V\" 123");
    assert_eq!(keys(&t), ["K"]);
    assert_eq!(s(t.get("K").unwrap()), "V");
}

#[test]
fn empty_first_key_is_replaced_by_the_next_string() {
    let t = read("\"\" \"K\" \"V\"");
    assert_eq!(keys(&t), ["K"]);
    assert_eq!(s(t.get("K").unwrap()), "V");
}

#[test]
fn keys_are_case_sensitive() {
    let t = read("\"a\" \"1\" \"A\" \"2\"");
    assert_eq!(keys(&t), ["a", "A"]);
}

#[test]
fn open_brace_without_key_throws_argument_null() {
    assert_eq!(read_err("{ \"X\" \"1\" }"), ExceptionKind::ArgumentNull);
}

#[test]
fn open_brace_after_string_value_throws_invalid_cast() {
    assert_eq!(read_err("\"T\" \"1\" \"T\" { }"), ExceptionKind::InvalidCast);
}

#[test]
fn array_length_of_a_string_throws_null_reference() {
    let t = read("\"A\" \"1\"");
    let mut c = KvtCursor::new(&t);
    assert_eq!(c.read_array_length("A").unwrap_err(), MonoException::null_reference());
    assert_eq!(c.read_array_length("Missing").unwrap(), 0);
}

#[test]
fn string_read_of_a_table_element_throws_invalid_cast() {
    let t = read("\"L\" [ { \"X\" \"1\" } ]");
    let mut c = KvtCursor::new(&t);
    c.read_key("L");
    c.read_array_index(0);
    assert_eq!(c.read_value().unwrap_err().kind, ExceptionKind::InvalidCast);
}

#[test]
fn list_index_out_of_range_throws() {
    let t = read("\"L\" [ \"a\" ]");
    let mut c = KvtCursor::new(&t);
    c.read_key("L");
    c.read_array_index(1);
    assert_eq!(c.read_value().unwrap_err(), MonoException::list_index_out_of_range());
}

#[test]
fn reading_without_a_key_throws_argument_null() {
    let t = read("\"A\" \"1\"");
    let mut c = KvtCursor::new(&t);
    assert_eq!(c.read_value().unwrap_err(), MonoException::argument_null("key"));
}

#[test]
fn cursor_state_belongs_to_the_table() {
    let t = read("\"T\" { \"A\" \"1\" \"B\" \"2\" }");
    let mut root = KvtCursor::new(&t);
    let mut first = root.read_object_key("T").unwrap().unwrap();
    first.read_key("B");
    let mut second = root.read_object_key("T").unwrap().unwrap();
    assert_eq!(second.read_value().unwrap().unwrap(), "2");
}

#[test]
fn int_reader_uses_number_styles_any_and_defaults_to_zero() {
    let t = read("\"A\" \"1,000\" \"B\" \"abc\"");
    let mut c = KvtCursor::new(&t);
    c.read_key("A");
    assert_eq!(c.read_i32().unwrap(), 1000);
    c.read_key("B");
    assert_eq!(c.read_i32().unwrap(), 0);
    c.read_key("Missing");
    assert_eq!(c.read_i32().unwrap(), 0);
    c.read_key("A");
    assert_eq!(c.read_u32().unwrap(), 1000);
}

#[test]
fn guid_reader() {
    let t = read("\"Z\" \"0\" \"E\" \"\" \"G\" \"53d3ea4a555c44d1824b495c2b337aec\" \"X\" \"xyz\"");
    let mut c = KvtCursor::new(&t);
    c.read_key("Z");
    assert_eq!(c.read_guid().unwrap(), Guid::EMPTY);
    c.read_key("E");
    assert_eq!(c.read_guid().unwrap(), Guid::EMPTY);
    c.read_key("Missing");
    assert_eq!(c.read_guid().unwrap(), Guid::EMPTY);
    c.read_key("G");
    assert_eq!(c.read_guid().unwrap().to_string_n(), "53d3ea4a555c44d1824b495c2b337aec");
    c.read_key("X");
    let err = c.read_guid().unwrap_err();
    assert_eq!((err.kind, err.message.as_str()), (ExceptionKind::Format, sr::FORMAT_GUID_INV_LEN));
}

#[test]
fn type_reader_chases_redirects_and_rejects_path_characters() {
    let old = "SDG.Framework.Foliage.KillVolume, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";
    let new = "SDG.Framework.Devkit.KillVolume, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";
    let text = format!("\"A\" \"{old}\" \"B\" \"Some/Path, Assembly\" \"C\" \"\"");
    let t = read(&text);
    let mut c = KvtCursor::new(&t);
    c.read_key("A");
    assert_eq!(c.read_type_name().unwrap().unwrap(), new);
    c.read_key("B");
    assert_eq!(c.read_type_name().unwrap(), None);
    c.read_key("C");
    assert_eq!(c.read_type_name().unwrap(), None);
}
