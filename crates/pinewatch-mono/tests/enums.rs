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

pinewatch_mono::net_enum! {
    /// A C# enum `: sbyte { LOW = -2, ZERO = 0, HIGH = 100 }`.
    struct Signed(i8) {
        LOW = -2,
        /// A documented member.
        ZERO = 0,
        HIGH = 100,
    }
}

pinewatch_mono::net_enum! {
    /// A C# `[Flags]` enum `: uint { NONE = 0, TOP = 1u << 31 }`.
    pub struct Unsigned(u32) {
        NONE = 0,
        TOP = 1 << 31
    }
}

/// `net_enum!` declares the constants and the members in declaration order, with values as Mono reads them into a
/// `ulong`: signed ones sign-extended, unsigned ones zero-extended.
#[test]
fn declared_enums() {
    assert_eq!((Signed::LOW, Signed::ZERO, Signed::HIGH), (Signed(-2), Signed(0), Signed(100)));
    assert_eq!(Signed::default(), Signed::ZERO);
    assert_eq!(Signed::INFO.underlying, EnumUnderlying::I8);
    assert_eq!(Signed::INFO.names, ["LOW", "ZERO", "HIGH"]);
    assert_eq!(Signed::INFO.values, [u64::MAX - 1, 0, 100]);
    assert_eq!(Signed::from_bits(u64::MAX - 1), Signed::LOW);
    assert_eq!(Signed::from_bits(0x1FF), Signed(-1));
    let signed = |s: &str| try_parse_enum(&Signed::INFO, Some(&NetString::from_str(s)), true).map(Signed::from_bits);
    assert_eq!(signed("low"), Some(Signed::LOW));
    assert_eq!(signed("-128"), Some(Signed(-128)));

    assert_eq!(Unsigned::TOP, Unsigned(0x8000_0000));
    assert_eq!(Unsigned::INFO.underlying, EnumUnderlying::U32);
    assert_eq!(Unsigned::INFO.names, ["NONE", "TOP"]);
    assert_eq!(Unsigned::INFO.values, [0, 0x8000_0000]);
    assert_eq!(Unsigned::from_bits(0x1_8000_0000), Unsigned::TOP);
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
fn equal_values_give_the_same_bits_in_any_order() {
    // V18: `Array.Sort` is unstable, even for three members: `[1, 1, 0]` reorders the two 1s. Members with equal
    // values give the same bits whichever of them matches first, so the result doesn't depend on stability.
    let info = EnumInfo { underlying: EnumUnderlying::I32, names: &["a", "A", "Z"], values: &[1, 1, 0] };
    let parse = |s: &str, ignore_case| try_parse_enum(&info, Some(&NetString::from_str(s)), ignore_case);
    assert_eq!(parse("a", true), Some(1));
    assert_eq!(parse("A", true), Some(1));
    assert_eq!(parse("A", false), Some(1));
    assert_eq!(parse("z", true), Some(0));
    assert_eq!(parse("a, Z", true), Some(1));

    // Above 16 members `Array.Sort` switches from insertion sort to introsort; the same holds.
    const NAMES: [&str; 20] = [
        "m19", "M18", "m17", "M17", "m16", "m15", "m14", "m13", "m12", "m11", "m10", "m9", "m8", "m7", "m6", "m5",
        "m4", "m3", "M3", "m0",
    ];
    const VALUES: [u64; 20] = [19, 18, 17, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 3, 0];
    let info = EnumInfo { underlying: EnumUnderlying::I32, names: &NAMES, values: &VALUES };
    let parse = |s: &str, ignore_case| try_parse_enum(&info, Some(&NetString::from_str(s)), ignore_case);
    assert_eq!(parse("M17", true), Some(17));
    assert_eq!(parse("m17", true), Some(17));
    assert_eq!(parse("m3", true), Some(3));
    assert_eq!(parse("m18", true), Some(18));
    assert_eq!(parse("m18", false), None);
}
