//! `UnityDatColorEx` (UnturnedDat/UnityDatColorEx.cs): colors as hexadecimal text with an optional `#`, or as a
//! dictionary with `R`, `G`, `B` (and `A`) keys.
//!
//! A failed `TryParseColor32RGB` hands back opaque black, not `default`, so the RGB forms return the result and the
//! value together. A failed `TryParseColor32RGBA` hands back `default`, so the RGBA forms return an `Option`.

use std::sync::OnceLock;

use pinewatch_mono::{NetString, NumberFormat, NumberStyles};
use pinewatch_unity::{Color, Color32};

use crate::tree::{DictRef, NodeRef, ValueRef};

const HASH: u16 = b'#' as u16;

/// `new Color32(0, 0, 0, byte.MaxValue)`, which a failed `TryParseColor32RGB` hands back.
const OPAQUE_BLACK: Color32 = Color32::new(0, 0, 0, u8::MAX);

/// `CultureInfo.InvariantCulture`'s number format.
fn invariant() -> &'static NumberFormat {
    static INVARIANT: OnceLock<NumberFormat> = OnceLock::new();
    INVARIANT.get_or_init(NumberFormat::invariant)
}

/// `byte.TryParse(value.Substring(start, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out result)`.
fn parse_hex_byte(value: &[u16], start: usize) -> Option<u8> {
    let text = NetString::from_units(value[start..start + 2].to_vec());
    // NumberStyles.HexNumber is a valid style, so this never fails with an exception.
    pinewatch_mono::try_parse_u8(Some(&text), NumberStyles::HEX_NUMBER, invariant()).ok().flatten()
}

impl ValueRef<'_> {
    /// `TryParseColor32RGB(out value)`: six hexadecimal digits with an optional `#` in front, opaque. On failure the
    /// value is opaque black.
    pub fn try_parse_color32_rgb(&self) -> (bool, Color32) {
        let Some(value) = self.value().filter(|value| !value.is_empty()) else {
            return (false, OPAQUE_BLACK);
        };
        let value = value.as_units();
        let start_index = if value[0] == HASH { 1 } else { 0 };
        if value.len() != 6 + start_index {
            // We require six characters for RGB color.
            return (false, OPAQUE_BLACK);
        }
        let Some(r) = parse_hex_byte(value, start_index) else {
            return (false, OPAQUE_BLACK);
        };
        let Some(g) = parse_hex_byte(value, start_index + 2) else {
            return (false, OPAQUE_BLACK);
        };
        let Some(b) = parse_hex_byte(value, start_index + 4) else {
            return (false, OPAQUE_BLACK);
        };
        (true, Color32::new(r, g, b, u8::MAX))
    }

    /// `ParseColor32RGB(defaultValue)`: on failure, `default` made opaque.
    pub fn parse_color32_rgb(&self, default: Color32) -> Color32 {
        match self.try_parse_color32_rgb() {
            (true, value) => value,
            (false, _) => Color32::new(default.r, default.g, default.b, u8::MAX),
        }
    }

    /// `TryParseColor32RGBA(out value)`: six or eight hexadecimal digits with an optional `#` in front. Without
    /// alpha digits the color is opaque.
    pub fn try_parse_color32_rgba(&self) -> Option<Color32> {
        let value = self.value().filter(|value| !value.is_empty())?.as_units();
        let start_index = if value[0] == HASH { 1 } else { 0 };
        let with_alpha = if value.len() == 8 + start_index {
            true
        } else if value.len() == 6 + start_index {
            false
        } else {
            // We require at least six characters for RGB color.
            return None;
        };
        let r = parse_hex_byte(value, start_index)?;
        let g = parse_hex_byte(value, start_index + 2)?;
        let b = parse_hex_byte(value, start_index + 4)?;
        let a = if with_alpha { parse_hex_byte(value, start_index + 6)? } else { u8::MAX };
        Some(Color32::new(r, g, b, a))
    }

    /// `ParseColor32RGBA(defaultValue)`.
    pub fn parse_color32_rgba(&self, default: Color32) -> Color32 {
        self.try_parse_color32_rgba().unwrap_or(default)
    }
}

impl DictRef<'_> {
    /// `TryParseColor32RGB(key, out value)`: the value under `key` as hexadecimal text, or a dictionary with `R`,
    /// `G` and `B` keys, which always succeeds (a channel that's missing or doesn't parse is 0). Opaque.
    pub fn try_parse_color32_rgb(&self, key: &str) -> (bool, Color32) {
        match self.try_get_node(key) {
            None | Some(NodeRef::List(_)) => (false, OPAQUE_BLACK),
            Some(NodeRef::Value(value_node)) => value_node.try_parse_color32_rgb(),
            Some(NodeRef::Dictionary(dictionary_node)) => {
                let r = dictionary_node.try_parse_uint8("R").unwrap_or_default();
                let g = dictionary_node.try_parse_uint8("G").unwrap_or_default();
                let b = dictionary_node.try_parse_uint8("B").unwrap_or_default();
                (true, Color32::new(r, g, b, u8::MAX))
            }
        }
    }

    /// `ParseColor32RGB(key, defaultValue)`: on failure, `default` made opaque.
    pub fn parse_color32_rgb(&self, key: &str, default: Color32) -> Color32 {
        match self.try_parse_color32_rgb(key) {
            (true, value) => value,
            (false, _) => Color32::new(default.r, default.g, default.b, u8::MAX),
        }
    }

    /// `TryParseColor32RGBA(key, out value)`: the value under `key` as hexadecimal text, or a dictionary with `R`,
    /// `G`, `B` and `A` keys, which always succeeds (a channel that's missing or doesn't parse is 0, alpha too).
    pub fn try_parse_color32_rgba(&self, key: &str) -> Option<Color32> {
        match self.try_get_node(key)? {
            NodeRef::List(_) => None,
            NodeRef::Value(value_node) => value_node.try_parse_color32_rgba(),
            NodeRef::Dictionary(dictionary_node) => {
                let r = dictionary_node.try_parse_uint8("R").unwrap_or_default();
                let g = dictionary_node.try_parse_uint8("G").unwrap_or_default();
                let b = dictionary_node.try_parse_uint8("B").unwrap_or_default();
                let a = dictionary_node.try_parse_uint8("A").unwrap_or_default();
                Some(Color32::new(r, g, b, a))
            }
        }
    }

    /// `ParseColor32RGBA(key, defaultValue)`.
    pub fn parse_color32_rgba(&self, key: &str, default: Color32) -> Color32 {
        self.try_parse_color32_rgba(key).unwrap_or(default)
    }

    /// `LegacyParseColor(key, defaultValue)` (`Data.readColor`): `try_parse_color32_rgb` converted to `Color` with
    /// alpha 1, or else the separate `key_R`, `key_G` and `key_B` floats with `default`'s channels where missing.
    pub fn legacy_parse_color(&self, key: &str, default: Color) -> Color {
        if let (true, value) = self.try_parse_color32_rgb(key) {
            let mut converted_color = Color::from(value);
            converted_color.a = 1.0;
            return converted_color;
        }
        Color::rgb(
            self.parse_float(&format!("{key}_R"), default.r),
            self.parse_float(&format!("{key}_G"), default.g),
            self.parse_float(&format!("{key}_B"), default.b),
        )
    }

    /// `LegacyParseColor32RGB(key, defaultValue)` (`Data.ReadColor32RGB`): `try_parse_color32_rgb`, or else the
    /// separate `key_R`, `key_G` and `key_B` bytes with `default`'s channels where missing. Opaque.
    pub fn legacy_parse_color32_rgb(&self, key: &str, default: Color32) -> Color32 {
        if let (true, value) = self.try_parse_color32_rgb(key) {
            return value;
        }
        Color32::new(
            self.parse_uint8(&format!("{key}_R"), default.r),
            self.parse_uint8(&format!("{key}_G"), default.g),
            self.parse_uint8(&format!("{key}_B"), default.b),
            u8::MAX,
        )
    }
}
