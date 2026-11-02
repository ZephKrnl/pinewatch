//! The value helpers: every row of `Assets/Tests/UnturnedDat/DatValueTests.cs` except the six `DateTime` rows
//! (`DateTime` parsing comes with the plan that needs it), one table per C# test method, plus the bool, number
//! style and type name rules.

mod common;

use common::{DatTestEnum, found, get_value, parse, tree_with_value};
use pinewatch_dat::{DatTree, TypeHandle, TypeResolver, ValueRef};
use pinewatch_mono::{ExceptionKind, Guid, MonoException, NetString};

/// `TryParseInt8`.
const TRY_PARSE_INT8: &[(&str, bool, i8)] = &[
    ("0", true, 0),
    ("-129", false, 0),
    ("-128", true, i8::MIN),
    ("1", true, 1),
    ("127", true, i8::MAX),
    ("x", false, 0),
];

/// `ParseInt8`.
const PARSE_INT8: &[(&str, i8, i8)] = &[
    ("0", 0, 9),
    ("-129", 3, 3),
    ("-128", i8::MIN, 0),
    ("1", 1, 0),
    ("127", i8::MAX, 0),
    ("x", 2, 2),
];

/// `TryParseUInt8`.
const TRY_PARSE_U_INT8: &[(&str, bool, u8)] = &[
    ("0", true, 0),
    ("-1", false, 0),
    ("1", true, 1),
    ("255", true, u8::MAX),
    ("x", false, 0),
];

/// `ParseUInt8`.
const PARSE_U_INT8: &[(&str, u8, u8)] = &[
    ("0", 0, 9),
    ("-1", 3, 3),
    ("1", 1, 0),
    ("255", u8::MAX, 0),
    ("x", 2, 2),
];

/// `TryParseInt16`.
const TRY_PARSE_INT16: &[(&str, bool, i16)] = &[
    ("0", true, 0),
    ("-32768", true, i16::MIN),
    ("-32769", false, 0),
    ("1", true, 1),
    ("32767", true, i16::MAX),
    ("32768", false, 0),
    ("x", false, 0),
];

/// `ParseInt16`.
const PARSE_INT16: &[(&str, i16, i16)] = &[
    ("0", 0, 9),
    ("-32768", -32768, 3),
    ("-32769", 3, 3),
    ("1", 1, 0),
    ("32767", 32767, 3),
    ("32768", 3, 3),
    ("x", 2, 2),
];

/// `TryParseUInt16`.
const TRY_PARSE_U_INT16: &[(&str, bool, u16)] = &[
    ("0", true, 0),
    ("-1", false, 0),
    ("1", true, 1),
    ("65535", true, u16::MAX),
    ("65536", false, 0),
    ("x", false, 0),
];

/// `ParseUInt16`.
const PARSE_U_INT16: &[(&str, u16, u16)] = &[
    ("0", 0, 9),
    ("-1", 3, 3),
    ("1", 1, 0),
    ("65535", u16::MAX, 0),
    ("65536", 3, 3),
    ("x", 2, 2),
];

/// `TryParseInt32`.
const TRY_PARSE_INT32: &[(&str, bool, i32)] = &[
    ("0", true, 0),
    ("-2,147,483,648", true, i32::MIN),
    ("-2,147,483,649", false, 0),
    ("1", true, 1),
    ("2,147,483,647", true, i32::MAX),
    ("2,147,483,648", false, 0),
    ("x", false, 0),
];

/// `ParseInt32`.
const PARSE_INT32: &[(&str, i32, i32)] = &[
    ("0", 0, 9),
    ("-2,147,483,648", i32::MIN, 3),
    ("-2,147,483,649", 3, 3),
    ("1", 1, 0),
    ("2,147,483,647", i32::MAX, 3),
    ("2,147,483,648", 3, 3),
    ("x", 2, 2),
];

/// `TryParseUInt32`.
const TRY_PARSE_U_INT32: &[(&str, bool, u32)] = &[
    ("0", true, 0),
    ("-1", false, 0),
    ("1", true, 1),
    ("4,294,967,295", true, u32::MAX),
    ("4,294,967,296", false, 0),
    ("x", false, 0),
];

/// `ParseUInt32`.
const PARSE_U_INT32: &[(&str, u32, u32)] = &[
    ("0", 0, 9),
    ("-1", 3, 3),
    ("1", 1, 0),
    ("4,294,967,295", u32::MAX, 0),
    ("4,294,967,296", 3, 3),
    ("x", 2, 2),
];

/// `TryParseInt64`.
const TRY_PARSE_INT64: &[(&str, bool, i64)] = &[
    ("0", true, 0),
    ("-9,223,372,036,854,775,808", true, i64::MIN),
    ("-9,223,372,036,854,775,809", false, 0),
    ("1", true, 1),
    ("9,223,372,036,854,775,807", true, i64::MAX),
    ("9,223,372,036,854,775,808", false, 0),
    ("x", false, 0),
];

/// `ParseInt64`.
const PARSE_INT64: &[(&str, i64, i64)] = &[
    ("0", 0, 9),
    ("-9,223,372,036,854,775,808", i64::MIN, 3),
    ("-9,223,372,036,854,775,809", 3, 3),
    ("1", 1, 0),
    ("9,223,372,036,854,775,807", i64::MAX, 3),
    ("9,223,372,036,854,775,808", 3, 3),
    ("x", 2, 2),
];

/// `TryParseUInt64`.
const TRY_PARSE_U_INT64: &[(&str, bool, u64)] = &[
    ("0", true, 0),
    ("-1", false, 0),
    ("1", true, 1),
    ("18,446,744,073,709,551,615", true, u64::MAX),
    ("18,446,744,073,709,551,616", false, 0),
    ("x", false, 0),
];

/// `ParseUInt64`.
const PARSE_U_INT64: &[(&str, u64, u64)] = &[
    ("0", 0, 9),
    ("-1", 3, 3),
    ("1", 1, 0),
    ("18,446,744,073,709,551,615", u64::MAX, 0),
    ("18,446,744,073,709,551,616", 3, 3),
    ("x", 2, 2),
];

/// `TryParseFloat`.
const TRY_PARSE_FLOAT: &[(&str, bool, f32)] = &[
    ("0", true, 0.0),
    ("0.0", true, 0.0),
    ("-1", true, -1.0),
    ("-1.0", true, -1.0),
    ("1", true, 1.0),
    ("1.0", true, 1.0),
    ("x", false, 0.0),
];

/// `ParseFloat`.
const PARSE_FLOAT: &[(&str, f32, f32)] = &[
    ("0", 0.0, 9.0),
    ("0.0", 0.0, 9.0),
    ("-1", -1.0, 9.0),
    ("-1.0", -1.0, 9.0),
    ("1", 1.0, 9.0),
    ("1.0", 1.0, 9.0),
    ("x", 2.0, 2.0),
];

/// `TryParseDouble`.
const TRY_PARSE_DOUBLE: &[(&str, bool, f64)] = &[
    ("0", true, 0.0),
    ("0.0", true, 0.0),
    ("-1", true, -1.0),
    ("-1.0", true, -1.0),
    ("1", true, 1.0),
    ("1.0", true, 1.0),
    ("x", false, 0.0),
];

/// `ParseDouble`.
const PARSE_DOUBLE: &[(&str, f64, f64)] = &[
    ("0", 0.0, 9.0),
    ("0.0", 0.0, 9.0),
    ("-1", -1.0, 9.0),
    ("-1.0", -1.0, 9.0),
    ("1", 1.0, 9.0),
    ("1.0", 1.0, 9.0),
    ("x", 2.0, 2.0),
];

/// `TryParseBool`.
const TRY_PARSE_BOOL: &[(&str, bool, bool)] = &[
    ("t", true, true),
    ("y", true, true),
    ("1", true, true),
    ("f", true, false),
    ("n", true, false),
    ("0", true, false),
    ("true", true, true),
    ("false", true, false),
    ("x", false, false),
    ("xyz", false, false),
];

/// `ParseBool`.
const PARSE_BOOL: &[(&str, bool, bool)] = &[
    ("t", true, false),
    ("f", false, true),
    ("x", false, false),
];

/// `TryParseEnumGeneric`.
const TRY_PARSE_ENUM_GENERIC: &[(&str, bool, DatTestEnum)] = &[
    ("None", true, DatTestEnum::NONE),
    ("Bear", true, DatTestEnum::BEAR),
    ("Dog", true, DatTestEnum::DOG),
    ("Other", false, DatTestEnum::NONE),
];

/// `TryParseEnumNonGeneric`.
const TRY_PARSE_ENUM_NON_GENERIC: &[(&str, bool, Option<DatTestEnum>)] = &[
    ("None", true, Some(DatTestEnum::NONE)),
    ("Bear", true, Some(DatTestEnum::BEAR)),
    ("Dog", true, Some(DatTestEnum::DOG)),
    ("Other", false, None),
];

/// `ParseEnumGeneric`.
const PARSE_ENUM_GENERIC: &[(&str, DatTestEnum, DatTestEnum)] = &[
    ("Bear", DatTestEnum::BEAR, DatTestEnum::DOG),
    ("Other", DatTestEnum::DOG, DatTestEnum::DOG),
];

/// `ParseEnumNonGeneric`.
const PARSE_ENUM_NON_GENERIC: &[(&str, DatTestEnum, DatTestEnum)] = &[
    ("Bear", DatTestEnum::BEAR, DatTestEnum::DOG),
    ("Other", DatTestEnum::DOG, DatTestEnum::DOG),
];

/// `TryParseGuid`.
const TRY_PARSE_GUID: &[(&str, bool, &str)] = &[
    ("6f65c8b5c7834d0c8898b881a667c527", true, "6f65c8b5c7834d0c8898b881a667c527"),
    ("00000000000000000000000000000000", true, "00000000000000000000000000000000"),
    ("xyz", false, "00000000000000000000000000000000"),
];

/// `ParseGuid`.
const PARSE_GUID: &[(&str, &str, &str)] = &[
    ("6f65c8b5c7834d0c8898b881a667c527", "6f65c8b5c7834d0c8898b881a667c527", "00000000000000000000000000000000"),
    ("00000000000000000000000000000000", "00000000000000000000000000000000", "6f65c8b5c7834d0c8898b881a667c527"),
    ("x", "6f65c8b5c7834d0c8898b881a667c527", "6f65c8b5c7834d0c8898b881a667c527"),
];

/// `ParseType`.
const PARSE_TYPE: &[(&str, Option<&str>, Option<&str>)] = &[
    ("System.Int32", Some("System.Int32"), None),
    ("abc", Some("System.Int32"), Some("System.Int32")),
    ("SDG.Unturned.IDatValue", Some("SDG.Unturned.IDatValue"), Some("System.Int32")),
];

/// Every C# test method, with its row count.
const TABLES: &[(&str, usize)] = &[
    ("TryParseInt8", TRY_PARSE_INT8.len()),
    ("ParseInt8", PARSE_INT8.len()),
    ("TryParseUInt8", TRY_PARSE_U_INT8.len()),
    ("ParseUInt8", PARSE_U_INT8.len()),
    ("TryParseInt16", TRY_PARSE_INT16.len()),
    ("ParseInt16", PARSE_INT16.len()),
    ("TryParseUInt16", TRY_PARSE_U_INT16.len()),
    ("ParseUInt16", PARSE_U_INT16.len()),
    ("TryParseInt32", TRY_PARSE_INT32.len()),
    ("ParseInt32", PARSE_INT32.len()),
    ("TryParseUInt32", TRY_PARSE_U_INT32.len()),
    ("ParseUInt32", PARSE_U_INT32.len()),
    ("TryParseInt64", TRY_PARSE_INT64.len()),
    ("ParseInt64", PARSE_INT64.len()),
    ("TryParseUInt64", TRY_PARSE_U_INT64.len()),
    ("ParseUInt64", PARSE_U_INT64.len()),
    ("TryParseFloat", TRY_PARSE_FLOAT.len()),
    ("ParseFloat", PARSE_FLOAT.len()),
    ("TryParseDouble", TRY_PARSE_DOUBLE.len()),
    ("ParseDouble", PARSE_DOUBLE.len()),
    ("TryParseBool", TRY_PARSE_BOOL.len()),
    ("ParseBool", PARSE_BOOL.len()),
    ("TryParseEnumGeneric", TRY_PARSE_ENUM_GENERIC.len()),
    ("TryParseEnumNonGeneric", TRY_PARSE_ENUM_NON_GENERIC.len()),
    ("ParseEnumGeneric", PARSE_ENUM_GENERIC.len()),
    ("ParseEnumNonGeneric", PARSE_ENUM_NON_GENERIC.len()),
    ("TryParseGuid", TRY_PARSE_GUID.len()),
    ("ParseGuid", PARSE_GUID.len()),
    ("ParseType", PARSE_TYPE.len()),
];

#[test]
fn every_row_is_ported() {
    let rows: usize = TABLES.iter().map(|&(_, rows)| rows).sum();
    assert_eq!(rows, 162, "{TABLES:?}");
}

/// The node `new DatValue(input)` makes.
fn value(tree: &DatTree) -> ValueRef<'_> {
    found(get_value(tree.root(), "key"), "the value", "key")
}

/// A `TryParseX` table, whose rows give the result and the out value (the type's default on failure), and its
/// `ParseX` table, whose rows give the result and the default passed in.
macro_rules! try_parse_and_parse {
    ($try_test:ident, $try_rows:ident, $try_parse:ident, $parse_test:ident, $parse_rows:ident, $parse:ident) => {
        #[test]
        fn $try_test() {
            for &(input, expected_success, expected_value) in $try_rows {
                let tree = tree_with_value(input);
                let actual_value = value(&tree).$try_parse();
                assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
                assert_eq!(expected_value, actual_value.unwrap_or_default(), "out value for {input:?}");
            }
        }

        #[test]
        fn $parse_test() {
            for &(input, expected_value, default_value) in $parse_rows {
                let tree = tree_with_value(input);
                assert_eq!(expected_value, value(&tree).$parse(default_value), "for {input:?}");
            }
        }
    };
}

try_parse_and_parse!(try_parse_int8, TRY_PARSE_INT8, try_parse_int8, parse_int8, PARSE_INT8, parse_int8);
try_parse_and_parse!(try_parse_u_int8, TRY_PARSE_U_INT8, try_parse_uint8, parse_u_int8, PARSE_U_INT8, parse_uint8);
try_parse_and_parse!(try_parse_int16, TRY_PARSE_INT16, try_parse_int16, parse_int16, PARSE_INT16, parse_int16);
try_parse_and_parse!(
    try_parse_u_int16,
    TRY_PARSE_U_INT16,
    try_parse_uint16,
    parse_u_int16,
    PARSE_U_INT16,
    parse_uint16
);
try_parse_and_parse!(try_parse_int32, TRY_PARSE_INT32, try_parse_int32, parse_int32, PARSE_INT32, parse_int32);
try_parse_and_parse!(
    try_parse_u_int32,
    TRY_PARSE_U_INT32,
    try_parse_uint32,
    parse_u_int32,
    PARSE_U_INT32,
    parse_uint32
);
try_parse_and_parse!(try_parse_int64, TRY_PARSE_INT64, try_parse_int64, parse_int64, PARSE_INT64, parse_int64);
try_parse_and_parse!(
    try_parse_u_int64,
    TRY_PARSE_U_INT64,
    try_parse_uint64,
    parse_u_int64,
    PARSE_U_INT64,
    parse_uint64
);
try_parse_and_parse!(try_parse_float, TRY_PARSE_FLOAT, try_parse_float, parse_float, PARSE_FLOAT, parse_float);
try_parse_and_parse!(try_parse_double, TRY_PARSE_DOUBLE, try_parse_double, parse_double, PARSE_DOUBLE, parse_double);
try_parse_and_parse!(try_parse_bool, TRY_PARSE_BOOL, try_parse_bool, parse_bool, PARSE_BOOL, parse_bool);
try_parse_and_parse!(
    try_parse_enum_generic,
    TRY_PARSE_ENUM_GENERIC,
    try_parse_enum,
    parse_enum_generic,
    PARSE_ENUM_GENERIC,
    parse_enum
);

/// `TryParseEnum(typeof(EDatTestEnum), out object)`: the same parse as the generic overload, but the out value is
/// null on failure. Rust has one generic `try_parse_enum`, whose `None` is that null.
#[test]
fn try_parse_enum_non_generic() {
    for &(input, expected_success, expected_value) in TRY_PARSE_ENUM_NON_GENERIC {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).try_parse_enum::<DatTestEnum>();
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        assert_eq!(expected_value, actual_value, "out value for {input:?}");
    }
}

/// `ParseEnum(typeof(EDatTestEnum), defaultValue)`.
#[test]
fn parse_enum_non_generic() {
    for &(input, expected_value, default_value) in PARSE_ENUM_NON_GENERIC {
        let tree = tree_with_value(input);
        assert_eq!(expected_value, value(&tree).parse_enum(default_value), "for {input:?}");
    }
}

/// `new System.Guid(string)`.
fn guid(text: &str) -> Guid {
    Guid::parse(&NetString::from(text)).expect("a valid GUID")
}

#[test]
fn try_parse_guid() {
    for &(input, expected_success, expected_value_string) in TRY_PARSE_GUID {
        let expected_value = guid(expected_value_string);
        let tree = tree_with_value(input);
        let actual_value = value(&tree).try_parse_guid();
        assert_eq!(expected_success, actual_value.is_some(), "TryParse result for {input:?}");
        assert_eq!(expected_value, actual_value.unwrap_or_default(), "out value for {input:?}");
    }
}

#[test]
fn parse_guid() {
    for &(input, expected_value_string, default_value_string) in PARSE_GUID {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).parse_guid(guid(default_value_string));
        assert_eq!(guid(expected_value_string), actual_value, "for {input:?}");
    }
}

/// Knows exactly the two types the C# rows name.
struct TestResolver;

impl TypeResolver for TestResolver {
    fn resolve(&self, assembly_qualified_name: &NetString) -> Result<Option<TypeHandle>, MonoException> {
        Ok(handle(&assembly_qualified_name.to_string_lossy()))
    }
}

fn handle(name: &str) -> Option<TypeHandle> {
    match name {
        "System.Int32" => Some(TypeHandle(1)),
        "SDG.Unturned.IDatValue" => Some(TypeHandle(2)),
        _ => None,
    }
}

/// `typeof(…)`, or null.
fn type_handle(name: Option<&str>) -> Option<TypeHandle> {
    name.map(|name| handle(name).expect("a type the test resolver knows"))
}

#[test]
fn parse_type() {
    for &(input, expected_value, default_value) in PARSE_TYPE {
        let tree = tree_with_value(input);
        let actual_value = value(&tree).parse_type(&TestResolver, type_handle(default_value));
        assert_eq!(Ok(type_handle(expected_value)), actual_value, "for {input:?}");
    }
}

/// `ParseType` returns the default for a null or empty value and for names containing `\`, `:` or `/`, without
/// asking the resolver.
#[test]
fn parse_type_rejects_invalid_names() {
    struct Anything;
    impl TypeResolver for Anything {
        fn resolve(&self, _: &NetString) -> Result<Option<TypeHandle>, MonoException> {
            Ok(Some(TypeHandle(7)))
        }
    }

    let fallback = Some(TypeHandle(1));
    for input in ["a\\b", "a:b", "a/b", ""] {
        let tree = tree_with_value(input);
        assert_eq!(Ok(fallback), value(&tree).parse_type(&Anything, fallback), "for {input:?}");
    }
    let (_, tree) = parse("key");
    assert_eq!(Ok(fallback), value(&tree).parse_type(&Anything, fallback), "for a flag");
    let tree = tree_with_value("a.b");
    assert_eq!(Ok(Some(TypeHandle(7))), value(&tree).parse_type(&Anything, fallback));
}

/// `Type.GetType` can throw even with `throwOnError: false`, and `ParseType` propagates that error.
#[test]
fn parse_type_propagates_get_type_errors() {
    struct Failing;
    impl TypeResolver for Failing {
        fn resolve(&self, _: &NetString) -> Result<Option<TypeHandle>, MonoException> {
            Err(MonoException::new(ExceptionKind::FileLoad, "Could not load file or assembly 'Some.Assembly'."))
        }
    }

    let tree = tree_with_value("Some.Type, Some.Assembly");
    assert_eq!(
        Err(MonoException::new(ExceptionKind::FileLoad, "Could not load file or assembly 'Some.Assembly'.")),
        value(&tree).parse_type(&Failing, None)
    );
}

/// `TryParseBool` reads a single character itself: `y`, `t` or `1` is true and `n`, `f` or `0` is false, case
/// sensitive, and any other single character fails. Longer text goes to `Boolean.TryParse`.
#[test]
fn bool_single_characters() {
    let cases = [
        ("y", Some(true)),
        ("t", Some(true)),
        ("1", Some(true)),
        ("n", Some(false)),
        ("f", Some(false)),
        ("0", Some(false)),
        ("Y", None),
        ("T", None),
        (" ", None),
        ("", None),
        ("yes", None),
        ("True", Some(true)),
        (" false ", Some(false)),
    ];
    for (input, expected) in cases {
        let tree = tree_with_value(input);
        assert_eq!(expected, value(&tree).try_parse_bool(), "for {input:?}");
    }
}

/// Numbers parse with `NumberStyles.Any` and the invariant culture: thousands separators, parentheses, exponents,
/// trailing signs, a zero fraction and the currency symbol `¤` are all accepted.
#[test]
fn integers_use_number_styles_any() {
    let cases = [
        ("1,000", Some(1000)),
        ("(5)", Some(-5)),
        ("1e3", Some(1000)),
        ("5-", Some(-5)),
        ("1.0", Some(1)),
        ("1.5", None),
        (" 7 ", Some(7)),
        ("\u{A4}5", Some(5)),
    ];
    for (input, expected) in cases {
        let tree = tree_with_value(input);
        assert_eq!(expected, value(&tree).try_parse_int32(), "for {input:?}");
    }
}

/// `IsValueNullOrEmpty`.
#[test]
fn is_value_null_or_empty() {
    let (_, tree) = parse("key");
    assert!(value(&tree).is_value_null_or_empty());
    assert!(value(&tree_with_value("")).is_value_null_or_empty());
    assert!(!value(&tree_with_value("x")).is_value_null_or_empty());
}
