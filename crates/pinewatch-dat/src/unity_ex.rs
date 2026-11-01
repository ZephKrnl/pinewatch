//! `UnityDatEx` (UnturnedDat/UnityDatEx.cs): vectors as `x, y[, z]` text, optionally in parentheses, or as a
//! dictionary with `X`, `Y` and `Z` keys.
//!
//! `IndexOf(')', openingParenthesisIndex + 2)` throws `ArgumentOutOfRangeException` when the value ends with `(`,
//! because that start index is past the end. The asset loader catches it, so these return `Err` there. Otherwise a
//! failed parse gives the C#'s `default`, which is `unwrap_or_default()`.

use std::sync::OnceLock;

use pinewatch_mono::{MonoException, NetString, NumberFormat, NumberStyles, sr};
use pinewatch_unity::{Vector2, Vector3};

use crate::tree::{DictRef, NodeRef, ValueRef};

const OPENING_PARENTHESIS: u16 = b'(' as u16;
const CLOSING_PARENTHESIS: u16 = b')' as u16;
const COMMA: u16 = b',' as u16;

/// `CultureInfo.InvariantCulture`'s number format.
fn invariant() -> &'static NumberFormat {
    static INVARIANT: OnceLock<NumberFormat> = OnceLock::new();
    INVARIANT.get_or_init(NumberFormat::invariant)
}

/// `value.IndexOf(c)`.
fn index_of(value: &[u16], c: u16) -> Option<usize> {
    value.iter().position(|&unit| unit == c)
}

/// `value.IndexOf(c, startIndex)`: throws when `startIndex` is past the end, like `String.IndexOf(char, int, int)`,
/// which it calls with the rest of the string as the count.
fn index_of_from(value: &[u16], c: u16, start_index: usize) -> Result<Option<usize>, MonoException> {
    let Some(rest) = value.get(start_index..) else {
        return Err(MonoException::argument_out_of_range_message("startIndex", sr::ARGUMENT_OUT_OF_RANGE_INDEX));
    };
    Ok(index_of(rest, c).map(|index| start_index + index))
}

/// `float.TryParse(text, NumberStyles.Any, CultureInfo.InvariantCulture, out result)`.
fn parse_float(text: &[u16]) -> Option<f32> {
    let text = NetString::from_units(text.to_vec());
    // NumberStyles.Any is a valid style, so this never fails with an exception.
    pinewatch_mono::try_parse_f32(Some(&text), NumberStyles::ANY, invariant()).ok().flatten()
}

/// The part between the parentheses, if any: `(startIndex, endIndex)` of the C#, `endIndex` inclusive. `None` when
/// there's an opening parenthesis without a closing one at least two characters later.
fn content_range(value: &[u16]) -> Result<Option<(usize, usize)>, MonoException> {
    match index_of(value, OPENING_PARENTHESIS) {
        Some(opening_parenthesis_index) => {
            let closing = index_of_from(value, CLOSING_PARENTHESIS, opening_parenthesis_index + 2)?;
            Ok(closing.map(|closing_parenthesis_index| (opening_parenthesis_index + 1, closing_parenthesis_index - 1)))
        }
        None => Ok(Some((0, value.len() - 1))),
    }
}

impl ValueRef<'_> {
    /// `TryParseVector2(out value)`: two comma-delimited floats, optionally in parentheses, for example `1, 2` or
    /// `(1, 2)`.
    pub fn try_parse_vector2(&self) -> Result<Option<Vector2>, MonoException> {
        let Some(value) = self.value().filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let value = value.as_units();
        let Some((start_index, end_index)) = content_range(value)? else {
            return Ok(None);
        };
        let Some(delimiter_index) = index_of_from(value, COMMA, start_index)? else {
            return Ok(None);
        };
        if delimiter_index + 1 > end_index {
            return Ok(None);
        }
        let Some(x) = parse_float(&value[start_index..delimiter_index]) else {
            return Ok(None);
        };
        let Some(y) = parse_float(&value[delimiter_index + 1..=end_index]) else {
            return Ok(None);
        };
        Ok(Some(Vector2::new(x, y)))
    }

    /// `ParseVector2(defaultValue)`.
    pub fn parse_vector2(&self, default: Vector2) -> Result<Vector2, MonoException> {
        Ok(self.try_parse_vector2()?.unwrap_or(default))
    }

    /// `TryParseVector3(out value)`: three comma-delimited floats, optionally in parentheses, for example `1, 2, 3`
    /// or `(1, 2, 3)`. The second comma is searched from two characters after the first.
    pub fn try_parse_vector3(&self) -> Result<Option<Vector3>, MonoException> {
        let Some(value) = self.value().filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let value = value.as_units();
        let Some((start_index, end_index)) = content_range(value)? else {
            return Ok(None);
        };
        let Some(first_delimiter_index) = index_of_from(value, COMMA, start_index)? else {
            return Ok(None);
        };
        if first_delimiter_index + 2 > end_index {
            return Ok(None);
        }
        let Some(second_delimiter_index) = index_of_from(value, COMMA, first_delimiter_index + 2)? else {
            return Ok(None);
        };
        if second_delimiter_index + 1 > end_index {
            return Ok(None);
        }
        let Some(x) = parse_float(&value[start_index..first_delimiter_index]) else {
            return Ok(None);
        };
        let Some(y) = parse_float(&value[first_delimiter_index + 1..second_delimiter_index]) else {
            return Ok(None);
        };
        let Some(z) = parse_float(&value[second_delimiter_index + 1..=end_index]) else {
            return Ok(None);
        };
        Ok(Some(Vector3::new(x, y, z)))
    }

    /// `ParseVector3(defaultValue)`.
    pub fn parse_vector3(&self, default: Vector3) -> Result<Vector3, MonoException> {
        Ok(self.try_parse_vector3()?.unwrap_or(default))
    }
}

impl DictRef<'_> {
    /// `TryParseVector2(key, out value)`: the value under `key` as text, or a dictionary with `X` and `Y` keys,
    /// which always succeeds (an axis that's missing or doesn't parse is 0).
    pub fn try_parse_vector2(&self, key: &str) -> Result<Option<Vector2>, MonoException> {
        match self.try_get_node(key) {
            None | Some(NodeRef::List(_)) => Ok(None),
            Some(NodeRef::Value(value_node)) => value_node.try_parse_vector2(),
            Some(NodeRef::Dictionary(dictionary_node)) => {
                let x = dictionary_node.try_parse_float("X").unwrap_or_default();
                let y = dictionary_node.try_parse_float("Y").unwrap_or_default();
                Ok(Some(Vector2::new(x, y)))
            }
        }
    }

    /// `ParseVector2(key, defaultValue)`.
    pub fn parse_vector2(&self, key: &str, default: Vector2) -> Result<Vector2, MonoException> {
        Ok(self.try_parse_vector2(key)?.unwrap_or(default))
    }

    /// `TryParseVector3(key, out value)`: the value under `key` as text, or a dictionary with `X`, `Y` and `Z` keys,
    /// which always succeeds (an axis that's missing or doesn't parse is 0).
    pub fn try_parse_vector3(&self, key: &str) -> Result<Option<Vector3>, MonoException> {
        match self.try_get_node(key) {
            None | Some(NodeRef::List(_)) => Ok(None),
            Some(NodeRef::Value(value_node)) => value_node.try_parse_vector3(),
            Some(NodeRef::Dictionary(dictionary_node)) => {
                let x = dictionary_node.try_parse_float("X").unwrap_or_default();
                let y = dictionary_node.try_parse_float("Y").unwrap_or_default();
                let z = dictionary_node.try_parse_float("Z").unwrap_or_default();
                Ok(Some(Vector3::new(x, y, z)))
            }
        }
    }

    /// `ParseVector3(key, defaultValue)`.
    pub fn parse_vector3(&self, key: &str, default: Vector3) -> Result<Vector3, MonoException> {
        Ok(self.try_parse_vector3(key)?.unwrap_or(default))
    }

    /// `LegacyParseVector3(key)` (`Data.readVector3`): `try_parse_vector3`, or else the separate `key_X`, `key_Y`
    /// and `key_Z` floats, 0 where missing.
    pub fn legacy_parse_vector3(&self, key: &str) -> Result<Vector3, MonoException> {
        if let Some(value) = self.try_parse_vector3(key)? {
            return Ok(value);
        }
        Ok(Vector3::new(
            self.parse_float(&format!("{key}_X"), 0.0),
            self.parse_float(&format!("{key}_Y"), 0.0),
            self.parse_float(&format!("{key}_Z"), 0.0),
        ))
    }
}
