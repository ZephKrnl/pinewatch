// The Latin-1 whitespace rule is ported from corefx `src/Common/src/CoreLib/System/Char.cs` (mono/corefx 5b4edb9b).
// Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.
// The case mapping follows `TextInfo.ToUpper(char)` and `ToLower(char)` in reference source `textinfo.cs` (MONO
// branch) and Mono's `ReferenceSources/TextInfo.cs` (Unity mono b8eb56f). MIT license; see NOTICES-PORTED-CODE.md.

//! `System.Char` classification and case mapping for one UTF-16 code unit, as Unity's Mono computes them.
//!
//! - `IsWhiteSpace` follows corefx: `IsWhiteSpaceLatin1` up to U+00FF, then the space, line and paragraph separator
//!   categories of `CharUnicodeInfoData` (mono/corefx 5b4edb9b), listed here.
//! - `ToUpperInvariant` and `ToLowerInvariant` are `CultureInfo.InvariantCulture.TextInfo.ToUpper` and `ToLower`:
//!   ASCII is folded directly, everything else comes from Mono's tables (`case_tables.rs`).
//!
//! These are ported from source; the harness confirms them against the shipped `mscorlib.dll`
//! (`docs/VERIFICATION.md`, V3).

use crate::case_tables::{TO_LOWER, TO_UPPER};

/// `char.IsWhiteSpace`.
pub fn is_white_space(c: u16) -> bool {
    if c < 0x100 {
        // `IsWhiteSpaceLatin1`.
        c == 0x20 || (0x09..=0x0D).contains(&c) || c == 0xA0 || c == 0x85
    } else {
        // `CharUnicodeInfo.IsWhiteSpace`: categories SpaceSeparator, LineSeparator and ParagraphSeparator.
        matches!(c, 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
    }
}

/// The table's mapping for `c`, or `c` itself.
fn mapped(table: &[(u16, u16)], c: u16) -> u16 {
    table.binary_search_by_key(&c, |&(from, _)| from).map_or(c, |index| table[index].1)
}

/// `char.ToLowerInvariant`: `TextInfo.ToLower(char)` for the invariant culture.
pub fn to_lower_invariant(c: u16) -> u16 {
    if c < 0x80 {
        // `ToLowerAsciiInvariant`.
        if (0x41..=0x5A).contains(&c) { c + 0x20 } else { c }
    } else {
        mapped(&TO_LOWER, c)
    }
}

/// `char.ToUpperInvariant`: `TextInfo.ToUpper(char)` for the invariant culture.
pub fn to_upper_invariant(c: u16) -> u16 {
    if c < 0x80 {
        // `ToUpperAsciiInvariant`.
        if (0x61..=0x7A).contains(&c) { c - 0x20 } else { c }
    } else {
        mapped(&TO_UPPER, c)
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
