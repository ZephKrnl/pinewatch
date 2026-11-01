//! `Data` (Assembly-CSharp/Unturned/Files/Data.cs): the line-based format of old `.dat` files, one `key value` per
//! line. Only the reading parts are ported (R7).
//!
//! Not ported yet: `readAssetReference`, `readMasterBundleReference` and `ReadAudioReference`, which need the asset
//! system and `Assets.currentMasterBundle`; the `hash` the constructors compute; and `readByteArray`, which no game
//! code calls.

use std::collections::HashMap;
use std::sync::OnceLock;

use pinewatch_mono::{Guid, MonoException, NetEnum, NetString, NumberFormat, NumberStyles, chars};
use pinewatch_unity::{Color, Color32, Vector3};

const SPACE: u16 = b' ' as u16;
const SLASH: u16 = b'/' as u16;
const CR: u16 = b'\r' as u16;
const LF: u16 = b'\n' as u16;

/// `CultureInfo.InvariantCulture`'s number format.
fn invariant() -> &'static NumberFormat {
    static INVARIANT: OnceLock<NumberFormat> = OnceLock::new();
    INVARIANT.get_or_init(NumberFormat::invariant)
}

/// A `readX(key, defaultValue)` that calls `X.TryParse(readString(key), NumberStyles.Any, InvariantCulture)`.
macro_rules! read_number {
    ($name:ident, $ty:ty, $parse:path, $csharp:literal) => {
        #[doc = concat!("`", $csharp, "(key, defaultValue)`: `NumberStyles.Any` with the invariant culture.")]
        pub fn $name(&self, key: &str, default: $ty) -> $ty {
            // NumberStyles.Any is a valid style, so this never fails with an exception.
            $parse(self.try_read_string(key), NumberStyles::ANY, invariant()).ok().flatten().unwrap_or(default)
        }
    };
}

/// `Data`.
#[derive(Debug, Default)]
pub struct LegacyData {
    /// `data`: a `Dictionary<string, string>` with the default comparer, so keys compare exactly. Nothing is removed
    /// while reading, so it enumerates in insertion order, and a duplicate key keeps its place.
    entries: Vec<(NetString, NetString)>,
    lookup: HashMap<Vec<u16>, usize>,
    /// `errors`: null until the first error.
    errors: Option<Vec<NetString>>,
}

impl LegacyData {
    /// `new Data(content)`: each line `StringReader.ReadLine` returns goes through `ParseLine`. The C# catches
    /// exceptions here, but none can occur for a non-null string.
    pub fn new(content: &NetString) -> Self {
        let mut data = Self::default();
        for line in read_lines(content.as_units()) {
            data.parse_line(line);
        }
        data
    }

    /// `errors`: `None` until an error was added.
    pub fn errors(&self) -> Option<&[NetString]> {
        self.errors.as_deref()
    }

    /// `isEmpty`.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `TryReadString(key, out value)`.
    pub fn try_read_string(&self, key: &str) -> Option<&NetString> {
        let key: Vec<u16> = key.encode_utf16().collect();
        self.lookup.get(key.as_slice()).map(|&index| &self.entries[index].1)
    }

    /// `readString(key, defaultValue)`.
    pub fn read_string<'a>(&'a self, key: &str, default: Option<&'a NetString>) -> Option<&'a NetString> {
        self.try_read_string(key).or(default)
    }

    /// `readEnum<T>(key, defaultValue)`: `Enum.Parse(typeof(T), value, ignoreCase: true)`, with `defaultValue` when it
    /// throws, which is when `Enum.TryParse` fails.
    pub fn read_enum<T: NetEnum>(&self, key: &str, default: T) -> T {
        match self.try_read_string(key) {
            Some(value) => pinewatch_mono::try_parse_enum(&T::INFO, Some(value), true).map_or(default, T::from_bits),
            None => default,
        }
    }

    /// `readBoolean(key, defaultValue)`: `y` or `true` ignoring case, or exactly `1`. The C# compares with
    /// `InvariantCultureIgnoreCase`, a linguistic comparison; this compares code units ignoring case
    /// (`docs/VERIFICATION.md`, V20).
    pub fn read_boolean(&self, key: &str, default: bool) -> bool {
        let Some(value) = self.try_read_string(key) else {
            return default;
        };
        let value = value.as_units();
        let y: Vec<u16> = "y".encode_utf16().collect();
        let one: Vec<u16> = "1".encode_utf16().collect();
        let true_text: Vec<u16> = "true".encode_utf16().collect();
        chars::equals_ordinal_ignore_case(value, &y)
            || value == one
            || chars::equals_ordinal_ignore_case(value, &true_text)
    }

    read_number!(read_byte, u8, pinewatch_mono::try_parse_u8, "readByte");
    read_number!(read_sbyte, i8, pinewatch_mono::try_parse_i8, "readSByte");
    read_number!(read_int16, i16, pinewatch_mono::try_parse_i16, "readInt16");
    read_number!(read_uint16, u16, pinewatch_mono::try_parse_u16, "readUInt16");
    read_number!(read_int32, i32, pinewatch_mono::try_parse_i32, "readInt32");
    read_number!(read_uint32, u32, pinewatch_mono::try_parse_u32, "readUInt32");
    read_number!(read_int64, i64, pinewatch_mono::try_parse_i64, "readInt64");
    read_number!(read_uint64, u64, pinewatch_mono::try_parse_u64, "readUInt64");
    read_number!(read_single, f32, pinewatch_mono::try_parse_f32, "readSingle");

    /// `readVector3(key)`: the floats under `key_X`, `key_Y` and `key_Z`, 0 where missing.
    pub fn read_vector3(&self, key: &str) -> Vector3 {
        Vector3::new(
            self.read_single(&format!("{key}_X"), 0.0),
            self.read_single(&format!("{key}_Y"), 0.0),
            self.read_single(&format!("{key}_Z"), 0.0),
        )
    }

    /// The angles `readQuaternion(key)` passes to `Quaternion.Euler`: twice the `key_X` byte, then the `key_Y` and
    /// `key_Z` bytes, 0 where missing. `Quaternion.Euler` is engine code, so the reader keeps its inputs (P6, L6).
    pub fn read_quaternion_euler_angles(&self, key: &str) -> Vector3 {
        Vector3::new(
            f32::from(self.read_byte(&format!("{key}_X"), 0)) * 2.0,
            f32::from(self.read_byte(&format!("{key}_Y"), 0)),
            f32::from(self.read_byte(&format!("{key}_Z"), 0)),
        )
    }

    /// `readColor(key)`: `readColor(key, Color.black)`.
    pub fn read_color(&self, key: &str) -> Color {
        self.read_color_with_default(key, Color::rgb(0.0, 0.0, 0.0))
    }

    /// `readColor(key, defaultColor)`: the floats under `key_R`, `key_G` and `key_B`, with `default`'s channels
    /// where missing. Alpha is 1.
    pub fn read_color_with_default(&self, key: &str, default: Color) -> Color {
        Color::rgb(
            self.read_single(&format!("{key}_R"), default.r),
            self.read_single(&format!("{key}_G"), default.g),
            self.read_single(&format!("{key}_B"), default.b),
        )
    }

    /// `ReadColor32RGB(key, defaultValue)`: the bytes under `key_R`, `key_G` and `key_B`, with `default`'s
    /// channels where missing. Opaque.
    pub fn read_color32_rgb(&self, key: &str, default: Color32) -> Color32 {
        Color32::new(
            self.read_byte(&format!("{key}_R"), default.r),
            self.read_byte(&format!("{key}_G"), default.g),
            self.read_byte(&format!("{key}_B"), default.b),
            u8::MAX,
        )
    }

    /// `readSteamID(key)`: the `CSteamID` holding `readUInt64(key)`, as its 64-bit value.
    pub fn read_steam_id(&self, key: &str) -> u64 {
        self.read_uint64(key, 0)
    }

    /// `readGUID(key)`: `Guid.Empty` for a missing, empty or `0` value, which allows null asset references.
    /// Otherwise `new Guid(value)`, which throws `FormatException` for text that isn't a GUID.
    pub fn read_guid(&self, key: &str) -> Result<Guid, MonoException> {
        match self.try_read_string(key) {
            Some(value) if !value.is_empty() && !is_zero(value) => Guid::parse(value),
            _ => Ok(Guid::default()),
        }
    }

    /// `ReadGuidOrLegacyId(key, out guid, out legacyId)`: a `ushort` comes first, because it fails for large GUID
    /// numbers, then a GUID. Missing, empty and `0` give neither.
    pub fn read_guid_or_legacy_id(&self, key: &str) -> (Guid, u16) {
        if let Some(value) = self.try_read_string(key).filter(|value| !value.is_empty() && !is_zero(value)) {
            // NumberStyles.Any is a valid style, so this never fails with an exception.
            let legacy_id = pinewatch_mono::try_parse_u16(Some(value), NumberStyles::ANY, invariant()).ok().flatten();
            if let Some(legacy_id) = legacy_id {
                return (Guid::default(), legacy_id);
            }
            if let Some(guid) = Guid::try_parse(Some(value)) {
                return (guid, 0);
            }
        }
        (Guid::default(), 0)
    }

    /// `getContents()`: the pairs in insertion order.
    pub fn get_contents(&self) -> &[(NetString, NetString)] {
        &self.entries
    }

    /// `getValuesWithKey(key)`: the values whose key equals `key` exactly.
    pub fn get_values_with_key(&self, key: &str) -> Vec<&NetString> {
        let key: Vec<u16> = key.encode_utf16().collect();
        self.entries.iter().filter(|(entry_key, _)| entry_key.as_units() == key).map(|(_, value)| value).collect()
    }

    /// `getKeysWithValue(value)`: the keys whose value equals `value` exactly.
    pub fn get_keys_with_value(&self, value: &str) -> Vec<&NetString> {
        let value: Vec<u16> = value.encode_utf16().collect();
        self.entries.iter().filter(|(_, entry_value)| entry_value.as_units() == value).map(|(key, _)| key).collect()
    }

    /// `has(key)`.
    pub fn has(&self, key: &str) -> bool {
        self.try_read_string(key).is_some()
    }

    /// `AddError`.
    fn add_error(&mut self, message: NetString) {
        self.errors.get_or_insert_with(Vec::new).push(message);
    }

    /// `ParseLine`: skips empty lines and lines starting with `//`, and splits the rest at the first space.
    fn parse_line(&mut self, line: &[u16]) {
        if line.is_empty() || (line.len() > 1 && line[0] == SLASH && line[1] == SLASH) {
            return;
        }

        let (key, value) = match line.iter().position(|&unit| unit == SPACE) {
            Some(split) => (&line[..split], &line[split + 1..]),
            None => (line, &line[line.len()..]),
        };

        if let Some(&index) = self.lookup.get(key) {
            let mut message: Vec<u16> = "Duplicate key: \"".encode_utf16().collect();
            message.extend_from_slice(key);
            message.extend("\" Old value: ".encode_utf16());
            message.extend_from_slice(self.entries[index].1.as_units());
            message.extend(" New value: ".encode_utf16());
            message.extend_from_slice(value);
            self.add_error(NetString::from_units(message));
            self.entries[index].1 = NetString::from_units(value.to_vec());
        } else {
            self.lookup.insert(key.to_vec(), self.entries.len());
            self.entries.push((NetString::from_units(key.to_vec()), NetString::from_units(value.to_vec())));
        }
    }
}

/// `value.Length == 1 && value[0] == '0'`.
fn is_zero(value: &NetString) -> bool {
    value.as_units() == [u16::from(b'0')]
}

/// The lines `StringReader.ReadLine` returns: each ends at `\r`, `\n` or `\r\n`, and text after the last line break
/// is a line only if it isn't empty.
fn read_lines(content: &[u16]) -> Vec<&[u16]> {
    let mut lines = Vec::new();
    let mut position = 0;
    while position < content.len() {
        match content[position..].iter().position(|&unit| unit == CR || unit == LF) {
            Some(offset) => {
                let end = position + offset;
                lines.push(&content[position..end]);
                position = end + 1;
                if content[end] == CR && content.get(position) == Some(&LF) {
                    position += 1;
                }
            }
            None => {
                lines.push(&content[position..]);
                position = content.len();
            }
        }
    }
    lines
}
