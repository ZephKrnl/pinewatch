//! `char.ToUpperInvariant`, `char.ToLowerInvariant` and `char.IsWhiteSpace` for every UTF-16 code unit, as Unity's
//! Mono computes them (`docs/VERIFICATION.md`, V3).
//!
//! Mono's `TextInfo.ToUpper(char)` and `ToLower(char)` (`mcs/class/corlib/ReferenceSources/TextInfo.cs` with the
//! `MONO` branch of reference source `textinfo.cs`, Unity mono `b8eb56f`) fold ASCII themselves and read everything
//! else from `TextInfoToUpperData` and `TextInfoToLowerData`. For the invariant culture they skip their special
//! cases (`µ`, `ı`, `ſ`, the title-case digraphs, `ς`, the Kelvin and Ångström signs, ...), so those stay unchanged.

use pinewatch_mono::chars::{equals_ordinal_ignore_case, is_white_space, to_lower_invariant, to_upper_invariant};

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// FNV-1a (64 bits) over the mapping of every code unit, each as two little-endian bytes.
fn checksum(map: impl Fn(u16) -> u16) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for c in 0..=u16::MAX {
        for byte in map(c).to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

#[test]
fn upper_invariant_rows() {
    for (c, upper) in [
        (0x0061, 0x0041), // ASCII
        (0x00FF, 0x0178), // ÿ -> Ÿ
        (0x00B5, 0x00B5), // µ: special case for other cultures only
        (0x0131, 0x0131), // ı
        (0x017F, 0x017F), // ſ
        (0x01C5, 0x01C5), // ǅ
        (0x01C6, 0x01C4), // ǆ -> Ǆ
        (0x019B, 0x019B), // ƛ: not in Mono's table
        (0x023F, 0x023F), // ȿ
        (0x0345, 0x0345),
        (0x03C2, 0x03C2), // ς
        (0x03D0, 0x03D0), // ϐ
        (0x1E9B, 0x1E9B),
        (0x1FBE, 0x1FBE),
        (0x1F80, 0x1F88), // ᾀ -> ᾈ
        (0x1FB3, 0x1FBC), // ᾳ -> ᾼ
        (0x2D00, 0x10A0), // ⴀ -> Ⴀ
        (0x1D79, 0xA77D), // the three single mappings outside the ranges
        (0x1D7D, 0x2C63),
        (0x214E, 0x2132),
        (0xFF41, 0xFF21), // fullwidth a
        (0x0587, 0x0587),
        (0xD800, 0xD800), // a lone surrogate is unchanged
    ] {
        assert_eq!(to_upper_invariant(c), upper, "U+{c:04X}");
    }
}

#[test]
fn lower_invariant_rows() {
    for (c, lower) in [
        (0x0041, 0x0061), // ASCII
        (0x00C0, 0x00E0), // À -> à
        (0x0130, 0x0130), // İ
        (0x01C5, 0x01C5), // ǅ
        (0x01C4, 0x01C6), // Ǆ -> ǆ
        (0x03A3, 0x03C3), // Σ -> σ
        (0x1E9E, 0x1E9E), // ẞ
        (0x2126, 0x2126), // Ω (ohm sign)
        (0x212A, 0x212A), // K (Kelvin sign)
        (0x212B, 0x212B), // Å (Ångström sign)
        (0x2132, 0x214E), // the two single mappings outside the ranges
        (0x2183, 0x2184),
        (0x10A0, 0x2D00), // Ⴀ -> ⴀ
        (0xFF21, 0xFF41), // fullwidth A
    ] {
        assert_eq!(to_lower_invariant(c), lower, "U+{c:04X}");
    }
}

/// The checksums and counts come from a model of the C# above, run on the source itself, independently of the
/// Rust tables.
#[test]
fn whole_tables_match_monos() {
    assert_eq!(checksum(to_upper_invariant), 0x48ed_4531_e939_9927);
    assert_eq!(checksum(to_lower_invariant), 0xb94e_c822_58b5_db7b);
    assert_eq!((0..=u16::MAX).filter(|&c| to_upper_invariant(c) != c).count(), 973);
    assert_eq!((0..=u16::MAX).filter(|&c| to_lower_invariant(c) != c).count(), 973);
}

#[test]
fn ordinal_ignore_case_folds_through_monos_table() {
    assert!(equals_ordinal_ignore_case(&units("Size"), &units("SIZE")));
    assert!(!equals_ordinal_ignore_case(&units("Size"), &units("\u{17F}ize")), "ſ isn't S");
    assert!(!equals_ordinal_ignore_case(&units("Item"), &units("\u{131}tem")), "ı isn't I");
    assert!(!equals_ordinal_ignore_case(&units("\u{B5}"), &units("\u{3BC}")), "µ isn't μ");
    assert!(!equals_ordinal_ignore_case(&units("\u{1C5}"), &units("\u{1C4}")), "ǅ isn't Ǆ");
    assert!(equals_ordinal_ignore_case(&units("\u{1F80}"), &units("\u{1F88}")), "ᾀ is ᾈ");
    assert!(equals_ordinal_ignore_case(&units("\u{2D00}"), &units("\u{10A0}")), "ⴀ is Ⴀ");
}

/// `IsWhiteSpace` above Latin-1 is the space, line and paragraph separator categories of corefx's
/// `CharUnicodeInfoData` (mono/corefx `5b4edb9b`): exactly these code units.
#[test]
fn white_space_above_latin1_is_corefxs_separators() {
    let found: Vec<u16> = (0x100..=u16::MAX).filter(|&c| is_white_space(c)).collect();
    let mut expected = vec![0x1680];
    expected.extend(0x2000..=0x200A);
    expected.extend([0x2028, 0x2029, 0x202F, 0x205F, 0x3000]);
    assert_eq!(found, expected);
}

#[test]
fn white_space_in_latin1() {
    let found: Vec<u16> = (0..0x100).filter(|&c| is_white_space(c)).collect();
    assert_eq!(found, [0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0x85, 0xA0]);
}
