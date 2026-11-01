//! The struct helpers: every row of `Assets/Tests/UnturnedDat/DatStructTests.cs`, one table per C# test method,
//! plus a parse that fails halfway.

mod common;

use common::{assert_no_error, found, get_list, parse};
use pinewatch_dat::{DatParseable, NodeRef};
use pinewatch_mono::{NumberFormat, NumberStyles, try_parse_i32};

/// `TestStruct`: `x,y` as a value, or `x` and `y` keys in a dictionary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TestStruct {
    x: i32,
    y: i32,
}

/// `int.TryParse(s, out result)`: `NumberStyles.Integer` with the current culture, which doesn't matter for these
/// rows, so the invariant culture stands in. Like the C# out parameter, `result` is 0 when parsing fails.
fn int_try_parse(text: &[u16], result: &mut i32) -> bool {
    let text = pinewatch_mono::NetString::from_units(text.to_vec());
    let parsed = try_parse_i32(Some(&text), NumberStyles::INTEGER, &NumberFormat::invariant()).ok().flatten();
    *result = parsed.unwrap_or(0);
    parsed.is_some()
}

impl DatParseable for TestStruct {
    fn try_parse(&mut self, node: NodeRef<'_>) -> bool {
        match node {
            NodeRef::Value(value) => {
                let Some(text) = value.value().filter(|text| !text.is_empty()) else {
                    return false;
                };
                let components: Vec<&[u16]> = text.as_units().split(|&unit| unit == u16::from(b',')).collect();
                if components.len() != 2 {
                    return false;
                }
                int_try_parse(components[0], &mut self.x) && int_try_parse(components[1], &mut self.y)
            }
            NodeRef::Dictionary(dict) => {
                self.x = dict.parse_int32("x", 0);
                self.y = dict.parse_int32("y", 0);
                true
            }
            NodeRef::List(_) => false,
        }
    }
}

/// `TryParseStruct`.
const TRY_PARSE_STRUCT: &[(&str, bool, i32, i32)] = &[
    ("key 1,2", true, 1, 2),
    ("key 1", false, 0, 0),
    ("key abc", false, 0, 0),
    ("key\n{\nx 1\ny 2\n}", true, 1, 2),
    ("key\n{\nx 1\n}", true, 1, 0),
    ("key\n{\ny 2\n}", true, 0, 2),
    ("key\n{\n}", true, 0, 0),
];

/// `ParseStruct`.
const PARSE_STRUCT: &[(&str, i32, i32, i32, i32)] = &[
    ("key 1,2", 1, 2, 0, 0),
    ("key abc", 3, 4, 3, 4),
    ("key\n{\nx 1\ny 2\n}", 1, 2, 0, 0),
    ("key\n{\nx 1\n}", 1, 0, 3, 4),
    ("key\n{\ny 2\n}", 0, 2, 3, 4),
    ("key\n{\n}", 0, 0, 3, 4),
    ("key", 3, 4, 3, 4),
];

/// `ParseListOfStructs`.
const PARSE_LIST_OF_STRUCTS: &[(&str, &[i32], &[i32])] = &[
    ("list\n[\n]", &[], &[]),
    ("list\n[\n1,2\n]", &[1], &[2]),
    ("list\n[\n1,2\n{\nx 3\ny 4\n}\n]", &[1, 3], &[2, 4]),
    ("list\n[\n1,2\ninvalid value\n{\nx 3\ny 4\n}\n]", &[1, 3], &[2, 4]),
];

/// `ParseArrayOfStructs`.
const PARSE_ARRAY_OF_STRUCTS: &[(&str, &[i32], &[i32])] = &[
    ("array\n[\n]", &[], &[]),
    ("array\n[\n1,2\n]", &[1], &[2]),
    ("array\n[\n1,2\n{\nx 3\ny 4\n}\n]", &[1, 3], &[2, 4]),
    ("array\n[\n1,2\ninvalid value\n{\nx 3\ny 4\n}\n]", &[1, 0, 3], &[2, 0, 4]),
];

/// `DictionaryParseListOfStructs`.
const DICTIONARY_PARSE_LIST_OF_STRUCTS: &[(&str, &[i32], &[i32])] = &[
    ("list\n[\n]", &[], &[]),
    ("list\n[\n1,2\n]", &[1], &[2]),
    ("list\n[\n1,2\n{\nx 3\ny 4\n}\n]", &[1, 3], &[2, 4]),
    ("list\n[\n1,2\ninvalid value\n{\nx 3\ny 4\n}\n]", &[1, 3], &[2, 4]),
];

/// `DictionaryParseArrayOfStructs`.
const DICTIONARY_PARSE_ARRAY_OF_STRUCTS: &[(&str, &[i32], &[i32])] = &[
    ("array\n[\n]", &[], &[]),
    ("array\n[\n1,2\n]", &[1], &[2]),
    ("array\n[\n1,2\n{\nx 3\ny 4\n}\n]", &[1, 3], &[2, 4]),
    ("array\n[\n1,2\ninvalid value\n{\nx 3\ny 4\n}\n]", &[1, 0, 3], &[2, 0, 4]),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("TryParseStruct", TRY_PARSE_STRUCT.len()),
    ("ParseStruct", PARSE_STRUCT.len()),
    ("ParseListOfStructs", PARSE_LIST_OF_STRUCTS.len()),
    ("ParseArrayOfStructs", PARSE_ARRAY_OF_STRUCTS.len()),
    ("DictionaryParseListOfStructs", DICTIONARY_PARSE_LIST_OF_STRUCTS.len()),
    ("DictionaryParseArrayOfStructs", DICTIONARY_PARSE_ARRAY_OF_STRUCTS.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 30, "{TABLES:?}");
}

#[test]
fn try_parse_struct() {
    for &(input, expected_success, expected_x, expected_y) in TRY_PARSE_STRUCT {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let (success, actual_value) = tree.root().try_parse_struct::<TestStruct>("key");
        assert_eq!(expected_success, success, "TryParseStruct for {input:?}");
        assert_eq!((expected_x, expected_y), (actual_value.x, actual_value.y), "for {input:?}");
    }
}

/// Default value is only used if `IDatParseable.TryParse` returns false.
#[test]
fn parse_struct() {
    for &(input, expected_x, expected_y, default_x, default_y) in PARSE_STRUCT {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let actual_value = tree.root().parse_struct("key", TestStruct { x: default_x, y: default_y });
        assert_eq!((expected_x, expected_y), (actual_value.x, actual_value.y), "for {input:?}");
    }
}

#[track_caller]
fn assert_structs(expected_x: &[i32], expected_y: &[i32], structs: &[TestStruct], input: &str) {
    assert_eq!(expected_x.len(), expected_y.len());
    let actual: Vec<(i32, i32)> = structs.iter().map(|value| (value.x, value.y)).collect();
    let expected: Vec<(i32, i32)> = expected_x.iter().copied().zip(expected_y.iter().copied()).collect();
    assert_eq!(expected, actual, "for {input:?}");
}

#[test]
fn parse_list_of_structs() {
    for &(input, expected_x, expected_y) in PARSE_LIST_OF_STRUCTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "list"), "TryGetList", input);
        assert_structs(expected_x, expected_y, &list.parse_list_of_structs::<TestStruct>(), input);
    }
}

#[test]
fn parse_array_of_structs() {
    for &(input, expected_x, expected_y) in PARSE_ARRAY_OF_STRUCTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let list = found(get_list(tree.root(), "array"), "TryGetList", input);
        let structs = list.parse_array_of_structs(TestStruct::default());
        assert_structs(expected_x, expected_y, &structs, input);
    }
}

#[test]
fn dictionary_parse_list_of_structs() {
    for &(input, expected_x, expected_y) in DICTIONARY_PARSE_LIST_OF_STRUCTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let structs = found(tree.root().parse_list_of_structs::<TestStruct>("list"), "the list", input);
        assert_structs(expected_x, expected_y, &structs, input);
    }
}

#[test]
fn dictionary_parse_array_of_structs() {
    for &(input, expected_x, expected_y) in DICTIONARY_PARSE_ARRAY_OF_STRUCTS {
        let (parser, tree) = parse(input);
        assert_no_error(&parser, input);
        let structs = found(tree.root().parse_array_of_structs("array", TestStruct::default()), "the list", input);
        assert_structs(expected_x, expected_y, &structs, input);
    }
}

/// `TryParseStruct` starts from `default` and keeps whatever `TryParse` filled in before it failed. `ParseStruct`
/// then returns the default instead, and the list helpers skip or replace the item.
#[test]
fn a_failed_parse_keeps_what_it_filled_in() {
    let (_, tree) = parse("key 1,x\nlist\n[\n1,x\n5,6\n]");
    assert_eq!((false, TestStruct { x: 1, y: 0 }), tree.root().try_parse_struct::<TestStruct>("key"));
    assert_eq!(TestStruct { x: 8, y: 9 }, tree.root().parse_struct("key", TestStruct { x: 8, y: 9 }));

    let list = found(get_list(tree.root(), "list"), "list", "partial");
    let (success, partial) = found(list.try_get_node(0), "list[0]", "partial").try_parse_struct::<TestStruct>();
    assert_eq!((false, TestStruct { x: 1, y: 0 }), (success, partial));
    assert_eq!(vec![TestStruct { x: 5, y: 6 }], list.parse_list_of_structs::<TestStruct>());
    let default = TestStruct { x: -1, y: -1 };
    assert_eq!(vec![default, TestStruct { x: 5, y: 6 }], list.parse_array_of_structs(default));

    assert_eq!((false, TestStruct::default()), tree.root().try_parse_struct::<TestStruct>("missing"));
    assert_eq!(None, tree.root().parse_list_of_structs::<TestStruct>("key"));
}
