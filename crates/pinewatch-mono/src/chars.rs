//! `System.Char` classification and case mapping for one UTF-16 code unit.
//!
//! ASCII and Latin-1 follow Mono exactly. Above Latin-1 these use Rust's Unicode data until the harness dumps
//! Mono's own tables (`docs/VERIFICATION.md`, "Unicode tables").

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

/// `char.IsNumber` (categories Nd, Nl, No).
pub fn is_number(c: u16) -> bool {
    if c < 0x80 { (0x30..=0x39).contains(&c) } else { as_char(c).is_some_and(char::is_numeric) }
}

/// `char.IsLetter` (categories Lu, Ll, Lt, Lm, Lo).
pub fn is_letter(c: u16) -> bool {
    match c {
        0x41..=0x5A | 0x61..=0x7A => true,
        0..=0x7F => false,
        0xAA | 0xB5 | 0xBA | 0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0xFF => true,
        0x80..=0xFF => false,
        _ => as_char(c).is_some_and(|ch| ch.is_alphabetic() && !ch.is_numeric()),
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

fn single_unit(mapped: Option<impl Iterator<Item = char>>) -> Option<u16> {
    let mut mapped = mapped?;
    let first = mapped.next()?;
    if mapped.next().is_some() {
        return None;
    }
    u16::try_from(u32::from(first)).ok()
}
