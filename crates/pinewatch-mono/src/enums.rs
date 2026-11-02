// Ported from the .NET Framework reference source `mscorlib/system/enum.cs` as Unity's Mono compiles it
// (Unity-Technologies/mono b8eb56f). Copyright (c) Microsoft Corporation. MIT license; see NOTICES-PORTED-CODE.md.

//! `Enum.TryParse(string value, bool ignoreCase, out TEnum result)`.
//!
//! A C# enum variable can hold any value of its underlying type, so a ported enum is a `#[repr(transparent)]`
//! struct around that type with associated constants, and implements [`NetEnum`].

use crate::{NetString, NumberFormat, NumberStyles, chars};

/// The underlying integer type of a C# enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumUnderlying {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
}

/// A C# enum's members in declaration order. Each value is stored as Mono reads it into a `ulong`: signed values
/// are sign-extended, so `-1` is `u64::MAX`.
#[derive(Debug, Clone, Copy)]
pub struct EnumInfo {
    pub underlying: EnumUnderlying,
    pub names: &'static [&'static str],
    pub values: &'static [u64],
}

/// A ported C# enum.
pub trait NetEnum: Copy {
    const INFO: EnumInfo;
    /// The enum value with these underlying bits (truncated to the underlying type, as `Enum.ToObject` does).
    fn from_bits(bits: u64) -> Self;
}

/// The members in the order `GetCachedValuesAndNames` returns them: Mono's `GetEnumValuesAndNames` gives
/// declaration order and reports whether the values ascend (unsigned); if not, `Array.Sort(values, names)` orders
/// them by value. `Array.Sort` is unstable (even `[1, 1, 0]` reorders its two 1s), but sorting by value fixes the
/// order of members with different values, and members with equal values give the same bits whichever matches
/// first, so any sort gives the same results (`VERIFICATION.md`, V18, settled).
fn ordered_members(info: &EnumInfo) -> Vec<(Vec<u16>, u64)> {
    let mut members: Vec<(Vec<u16>, u64)> =
        info.names.iter().zip(info.values).map(|(name, &value)| (name.encode_utf16().collect(), value)).collect();
    let sorted = info.values.windows(2).all(|pair| pair[0] <= pair[1]);
    if !sorted {
        members.sort_by_key(|&(_, value)| value);
    }
    members
}

/// `Convert.ChangeType(value, underlyingType, CultureInfo.InvariantCulture)`: the underlying type's `Parse` with
/// `NumberStyles.Integer`. The result is sign-extended to 64 bits like the member values.
fn parse_underlying(underlying: EnumUnderlying, value: &NetString) -> Option<u64> {
    let invariant = NumberFormat::invariant();
    let style = NumberStyles::INTEGER;
    let value = Some(value);
    // NumberStyles.Integer is always a valid style, so these never return Err.
    match underlying {
        EnumUnderlying::I8 => crate::try_parse_i8(value, style, &invariant).ok().flatten().map(|v| v as u64),
        EnumUnderlying::U8 => crate::try_parse_u8(value, style, &invariant).ok().flatten().map(u64::from),
        EnumUnderlying::I16 => crate::try_parse_i16(value, style, &invariant).ok().flatten().map(|v| v as u64),
        EnumUnderlying::U16 => crate::try_parse_u16(value, style, &invariant).ok().flatten().map(u64::from),
        EnumUnderlying::I32 => crate::try_parse_i32(value, style, &invariant).ok().flatten().map(|v| v as u64),
        EnumUnderlying::U32 => crate::try_parse_u32(value, style, &invariant).ok().flatten().map(u64::from),
        EnumUnderlying::I64 => crate::try_parse_i64(value, style, &invariant).ok().flatten().map(|v| v as u64),
        EnumUnderlying::U64 => crate::try_parse_u64(value, style, &invariant).ok().flatten(),
    }
}

/// `Enum.TryParse` (`TryParseEnum` without throwing): the raw value bits, or `None`. A missing string fails.
pub fn try_parse_enum(info: &EnumInfo, value: Option<&NetString>, ignore_case: bool) -> Option<u64> {
    let value = chars::trim(value?.as_units());
    if value.is_empty() {
        return None;
    }

    // When the number doesn't parse, the C# falls through to the name lookup after a FormatException and fails
    // after an OverflowException. A string starting with a digit or a sign never matches a member name (C#
    // identifiers can't start with one), so both fail. `char.IsDigit` also accepts non-ASCII digits, which can't
    // parse as a number and can't start a name either, so testing ASCII digits gives the same results.
    let first = value[0];
    if ((u16::from(b'0')..=u16::from(b'9')).contains(&first) || first == u16::from(b'-') || first == u16::from(b'+'))
        && let Some(bits) = parse_underlying(info.underlying, &NetString::from_units(value.to_vec()))
    {
        return Some(bits);
    }

    let members = ordered_members(info);
    let mut result: u64 = 0;
    for part in value.split(|&c| c == u16::from(b',')) {
        let part = chars::trim(part);
        let found = members.iter().find(|(name, _)| {
            if ignore_case { chars::equals_ordinal_ignore_case(name, part) } else { name.as_slice() == part }
        });
        result |= found?.1;
    }
    Some(result)
}
