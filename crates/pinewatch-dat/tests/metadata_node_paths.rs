//! Node paths: every row of `Assets/Tests/UnturnedDat/DatParserMetadataNodePathTests.cs`, one table per C# test
//! method, plus parents, nested paths and nodes parsed without metadata.

mod common;

use common::{
    assert_no_error, found, get_dictionary, get_list, get_value, list_get_dictionary, list_get_value, parse,
    parse_with_metadata,
};
use pinewatch_dat::NodeRef;
use pinewatch_mono::NetString;

/// `RootDictionaryPath`.
const ROOT_DICTIONARY_PATH: &[(&str, &str)] = &[
    ("Key1 Value1\nKey2 Value2\nKey3 Value3", "Key1"),
    ("Key1 Value1\nKey2 Value2\nKey3 Value3", "Key2"),
    ("Key1 Value1\nKey2 Value2\nKey3 Value3", "Key3"),
];

/// `ListPath`.
const LIST_PATH: &[(&str, i32, &str)] = &[
    ("List\n[\nValue1\nValue2\nValue3\n]", 0, "/List/0"),
    ("List\n[\nValue1\nValue2\nValue3\n]", 1, "/List/1"),
    ("List\n[\nValue1\nValue2\nValue3\n]", 2, "/List/2"),
];

/// `DictionaryPath`.
const DICTIONARY_PATH: &[(&str, &str, &str)] = &[
    ("Dict\n{\nKey1 Value1\nKey2 Value2\nKey3 Value3\n}", "Key1", "/Dict/Key1"),
    ("Dict\n{\nKey1 Value1\nKey2 Value2\nKey3 Value3\n}", "Key2", "/Dict/Key2"),
    ("Dict\n{\nKey1 Value1\nKey2 Value2\nKey3 Value3\n}", "Key3", "/Dict/Key3"),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("RootDictionaryPath", ROOT_DICTIONARY_PATH.len()),
    ("ListPath", LIST_PATH.len()),
    ("DictionaryPath", DICTIONARY_PATH.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 9, "{TABLES:?}");
}

/// `Assert.IsTrue(node.TryGetNodePath(out string actualPath))` and `Assert.AreEqual(expectedPath, actualPath)`.
#[track_caller]
fn assert_path(expected_path: &str, node: NodeRef<'_>, input: &str) {
    assert_eq!(Some(NetString::from(expected_path)), node.node_path(), "for {input:?}");
}

#[test]
fn root_dictionary_path() {
    for &(input, key) in ROOT_DICTIONARY_PATH {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let node = found(get_value(tree.root(), key), "TryGetValue", input);
        assert_path(&format!("/{key}"), node.into(), input);
    }
}

#[test]
fn list_path() {
    for &(input, index, expected_path) in LIST_PATH {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "List"), "TryGetList", input);
        let node = found(list_get_value(list, index), "TryGetValue", input);
        assert_path(expected_path, node.into(), input);
    }
}

#[test]
fn dictionary_path() {
    for &(input, key, expected_path) in DICTIONARY_PATH {
        let (parser, tree) = parse_with_metadata(input);
        assert_no_error(&parser, input);
        let child_dictionary = found(get_dictionary(tree.root(), "Dict"), "TryGetDictionary", input);
        let node = found(get_value(child_dictionary, key), "TryGetValue", input);
        assert_path(expected_path, node.into(), input);
    }
}

/// The root's parent is null, so its path is empty.
#[test]
fn root_path_is_empty() {
    let (_, tree) = parse_with_metadata("a 1");
    let root = NodeRef::from(tree.root());
    assert!(root.is_metadata_available());
    assert!(root.parent().is_none());
    assert_path("", root, "root");
}

/// The example from the C# test's summary: an error deep in a blueprint list.
#[test]
fn nested_path() {
    let input = "Blueprints\n[\n{\nInputItems\n[\n{\n}\n{\nErrorHere x\n}\n]\n}\n]";
    let (parser, tree) = parse_with_metadata(input);
    assert_no_error(&parser, input);
    let blueprints = found(get_list(tree.root(), "Blueprints"), "Blueprints", input);
    let blueprint = found(list_get_dictionary(blueprints, 0), "Blueprints/0", input);
    let input_items = found(get_list(blueprint, "InputItems"), "InputItems", input);
    let item = found(list_get_dictionary(input_items, 1), "InputItems/1", input);
    let error_here = found(get_value(item, "ErrorHere"), "ErrorHere", input);

    assert_path("/Blueprints/0/InputItems/1/ErrorHere", error_here.into(), input);
    assert_path("/Blueprints/0/InputItems/1", item.into(), input);
    let parent = found(NodeRef::from(error_here).parent(), "parent", input);
    assert_eq!(item.id(), parent.id());
    let grandparent = found(parent.parent(), "grandparent", input);
    assert_eq!(input_items.id(), grandparent.id());
}

/// A path names the key as first spelled, whichever spelling replaced the value.
#[test]
fn path_uses_the_first_spelling() {
    let (_, tree) = parse_with_metadata("Key 1\nKEY 2");
    let node = found(get_value(tree.root(), "key"), "key", "spelling");
    assert_path("/Key", node.into(), "spelling");
}

/// Without metadata no node has a parent or a path.
#[test]
fn no_paths_without_metadata() {
    let (_, tree) = parse("a 1\nd\n{\nk v\n}");
    let dictionary = found(get_dictionary(tree.root(), "d"), "d", "plain");
    let k = NodeRef::from(found(get_value(dictionary, "k"), "k", "plain"));
    assert!(!k.is_metadata_available());
    assert!(k.parent().is_none());
    assert_eq!(None, k.node_path());
    assert_eq!(None, NodeRef::from(tree.root()).node_path());
}
