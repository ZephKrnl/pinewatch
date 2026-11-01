// The Latin-1 whitespace rule is ported from corefx `src/Common/src/CoreLib/System/Char.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

//! `System.Char` classification and case mapping for one UTF-16 code unit.
//!
//! - `IsWhiteSpace` follows corefx exactly up to U+00FF (`IsWhiteSpaceLatin1`). Above that it uses Rust's
//!   `White_Space` property, which for current Unicode data is the same set as .NET's space, line and paragraph
//!   separator categories.
//! - Case mapping is exact for ASCII. Above ASCII it uses Rust's mappings when they map to a single code unit and
//!   leaves the character unchanged otherwise. .NET uses simple mappings, so a few characters differ (for example
//!   U+0130 lowercases to U+0069 in .NET but stays U+0130 here).
//!
//! Both gaps wait for the harness to dump Mono's tables (`docs/VERIFICATION.md`, V3).

fn as_char(c: u16) -> Option<char> {
    char::from_u32(u32::from(c))
}

/// `char.IsWhiteSpace`.
pub fn is_white_space(c: u16) -> bool {
    if c < 0x100 {
        c == 0x20 || (0x09..=0x0D).contains(&c) || c == 0xA0 || c == 0x85
    } else {
        as_char(c).is_some_and(char::is_whitespace)
    }
}

/// `char.ToLowerInvariant`.
pub fn to_lower_invariant(c: u16) -> u16 {
    if c < 0x80 {
        if (0x41..=0x5A).contains(&c) { c + 0x20 } else { c }
    } else {
        single_unit(as_char(c).map(|ch| ch.to_lowercase())).unwrap_or(c)
    }
}

/// `char.ToUpperInvariant`.
pub fn to_upper_invariant(c: u16) -> u16 {
    if c < 0x80 {
        if (0x61..=0x7A).contains(&c) { c - 0x20 } else { c }
    } else {
        single_unit(as_char(c).map(|ch| ch.to_uppercase())).unwrap_or(c)
    }
}

/// `StringComparison.OrdinalIgnoreCase` equality: the same length, and every code unit equal after
/// `ToUpperInvariant`.
pub fn equals_ordinal_ignore_case(a: &[u16], b: &[u16]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| x == y || to_upper_invariant(x) == to_upper_invariant(y))
}

/// `string.Trim()` and `ReadOnlySpan<char>.Trim()`: removes `char.IsWhiteSpace` characters from both ends.
pub fn trim(s: &[u16]) -> &[u16] {
    let start = s.iter().position(|&c| !is_white_space(c)).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| !is_white_space(c)).map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

fn single_unit(mapped: Option<impl Iterator<Item = char>>) -> Option<u16> {
    let mut mapped = mapped?;
    let first = mapped.next()?;
    if mapped.next().is_some() {
        return None;
    }
    u16::try_from(u32::from(first)).ok()
}
