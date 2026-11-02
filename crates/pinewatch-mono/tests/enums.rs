//! `Enum.TryParse` (reference source `enum.cs` over Mono's `GetEnumValuesAndNames`).

use pinewatch_mono::{EnumInfo, EnumUnderlying, NetEnum, NetString, try_parse_enum};

/// A C# enum `{ A = 0, B = 1, C = 4 }` with a `uint` underlying type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
struct Test(u32);

impl NetEnum for Test {
    const INFO: EnumInfo = EnumInfo { underlying: EnumUnderlying::U32, names: &["A", "B", "C"], values: &[0, 1, 4] };

    fn from_bits(bits: u64) -> Self {
        Self(bits as u32)
    }
}

fn parse(s: &str, ignore_case: bool) -> Option<Test> {
    try_parse_enum(&Test::INFO, Some(&NetString::from_str(s)), ignore_case).map(Test::from_bits)
}

#[test]
fn enum_parsing() {
    assert_eq!(parse("B", true), Some(Test(1)));
    assert_eq!(parse("b", true), Some(Test(1)));
    assert_eq!(parse("b", false), None);
    assert_eq!(parse("4", false), Some(Test(4)));
    assert_eq!(parse("7", false), Some(Test(7)));
    assert_eq!(parse("B, C", false), Some(Test(5)));
    assert_eq!(parse(" B ", false), Some(Test(1)));
    assert_eq!(parse("-1", false), None);
    assert_eq!(parse("", false), None);
    assert_eq!(parse("X", false), None);
    assert_eq!(try_parse_enum(&Test::INFO, None, true), None);
}

#[test]
fn unsorted_values_are_looked_up_in_value_order() {
    // Mono returns the members in declaration order and enum.cs sorts them by value when they aren't ascending,
    // so with ignoreCase the first match is the member with the lower value.
    let info = EnumInfo { underlying: EnumUnderlying::I32, names: &["b", "B"], values: &[5, 1] };
    let parse = |s: &str, ignore_case| try_parse_enum(&info, Some(&NetString::from_str(s)), ignore_case);
    assert_eq!(parse("b", true), Some(1));
    assert_eq!(parse("b", false), Some(5));
}

#[test]
fn equal_valued_members_are_interchangeable() {
    // `Array.Sort` may reorder members with equal values. They contribute the same bits, so the result doesn't
    // depend on which equal-valued name the lookup finds first (`VERIFICATION.md`, V18).
    let info = EnumInfo { underlying: EnumUnderlying::I32, names: &["b", "B", "a"], values: &[1, 1, 0] };
    let parse = |s: &str| try_parse_enum(&info, Some(&NetString::from_str(s)), true);
    assert_eq!(parse("b"), Some(1));
    assert_eq!(parse("B"), Some(1));
    assert_eq!(parse("a"), Some(0));
}
