//! `DatValueEx` (UnturnedDat/DatValueEx.cs): reading a value node as a number, bool, GUID, enum or type.
//!
//! Each `try_parse_x` is `TryParseX(out value)`: `None` where the C# returns false. On failure the C# out value is
//! always the type's default, which is `unwrap_or_default()`. Each `parse_x(default)` is `ParseX(defaultValue)`.

use std::sync::OnceLock;

use pinewatch_mono::{Guid, NetEnum, NumberFormat, NumberStyles};

use crate::tree::ValueRef;
use crate::types::{INVALID_TYPE_CHARS, TypeHandle, TypeResolver};

const LOWER_Y: u16 = b'y' as u16;
const LOWER_T: u16 = b't' as u16;
const ONE: u16 = b'1' as u16;
const LOWER_N: u16 = b'n' as u16;
const LOWER_F: u16 = b'f' as u16;
const ZERO: u16 = b'0' as u16;

/// `CultureInfo.InvariantCulture`'s number format.
fn invariant() -> &'static NumberFormat {
    static INVARIANT: OnceLock<NumberFormat> = OnceLock::new();
    INVARIANT.get_or_init(NumberFormat::invariant)
}

/// A `TryParseX`/`ParseX` pair that calls `X.TryParse(value, NumberStyles.Any, CultureInfo.InvariantCulture)`.
macro_rules! number_pair {
    ($try_name:ident, $name:ident, $ty:ty, $parse:path, $csharp:literal) => {
        #[doc = concat!("`", $csharp, ".TryParse(value, NumberStyles.Any, CultureInfo.InvariantCulture)`.")]
        pub fn $try_name(&self) -> Option<$ty> {
            // NumberStyles.Any is a valid style, so this never fails with an exception.
            $parse(self.value(), NumberStyles::ANY, invariant()).ok().flatten()
        }

        #[doc = concat!("The `", stringify!($try_name), "` result, or `default`.")]
        pub fn $name(&self, default: $ty) -> $ty {
            self.$try_name().unwrap_or(default)
        }
    };
}

impl ValueRef<'_> {
    /// `IsValueNullOrEmpty`.
    pub fn is_value_null_or_empty(&self) -> bool {
        self.value().is_none_or(|value| value.is_empty())
    }

    number_pair!(try_parse_int8, parse_int8, i8, pinewatch_mono::try_parse_i8, "sbyte");
    number_pair!(try_parse_uint8, parse_uint8, u8, pinewatch_mono::try_parse_u8, "byte");
    number_pair!(try_parse_int16, parse_int16, i16, pinewatch_mono::try_parse_i16, "short");
    number_pair!(try_parse_uint16, parse_uint16, u16, pinewatch_mono::try_parse_u16, "ushort");
    number_pair!(try_parse_int32, parse_int32, i32, pinewatch_mono::try_parse_i32, "int");
    number_pair!(try_parse_uint32, parse_uint32, u32, pinewatch_mono::try_parse_u32, "uint");
    number_pair!(try_parse_int64, parse_int64, i64, pinewatch_mono::try_parse_i64, "long");
    number_pair!(try_parse_uint64, parse_uint64, u64, pinewatch_mono::try_parse_u64, "ulong");
    number_pair!(try_parse_float, parse_float, f32, pinewatch_mono::try_parse_f32, "float");
    number_pair!(try_parse_double, parse_double, f64, pinewatch_mono::try_parse_f64, "double");

    /// `TryParseEnum<T>`: `Enum.TryParse(value, ignoreCase: true, out result)`. The C# overload that takes a
    /// `System.Type` does the same parse and gives null on failure, which is this `None`.
    pub fn try_parse_enum<T: NetEnum>(&self) -> Option<T> {
        pinewatch_mono::try_parse_enum(&T::INFO, self.value(), true).map(T::from_bits)
    }

    /// The `try_parse_enum` result, or `default`.
    pub fn parse_enum<T: NetEnum>(&self, default: T) -> T {
        self.try_parse_enum().unwrap_or(default)
    }

    /// `TryParseBool`: a single character is read here, case-sensitively: `y`, `t` and `1` are true, `n`, `f` and
    /// `0` are false, and any other single character fails. Longer text goes to `Boolean.TryParse`.
    pub fn try_parse_bool(&self) -> Option<bool> {
        let value = self.value().filter(|value| !value.is_empty())?;
        if let [letter] = value.as_units() {
            return match *letter {
                LOWER_Y | LOWER_T | ONE => Some(true),
                LOWER_N | LOWER_F | ZERO => Some(false),
                _ => None,
            };
        }
        pinewatch_mono::try_parse_bool(Some(value))
    }

    /// The `try_parse_bool` result, or `default`.
    pub fn parse_bool(&self, default: bool) -> bool {
        self.try_parse_bool().unwrap_or(default)
    }

    /// `TryParseGuid`: `Guid.TryParse(value)`.
    pub fn try_parse_guid(&self) -> Option<Guid> {
        Guid::try_parse(self.value())
    }

    /// The `try_parse_guid` result, or `default`.
    pub fn parse_guid(&self, default: Guid) -> Guid {
        self.try_parse_guid().unwrap_or(default)
    }

    /// `ParseType(defaultValue)`: `default` for a null or empty value or a name containing `\`, `:` or `/`;
    /// otherwise the type the resolver finds, or `default`.
    pub fn parse_type(&self, resolver: &dyn TypeResolver, default: Option<TypeHandle>) -> Option<TypeHandle> {
        let Some(value) = self.value().filter(|value| !value.is_empty()) else {
            return default;
        };
        if value.as_units().iter().any(|unit| INVALID_TYPE_CHARS.contains(unit)) {
            return default;
        }
        resolver.resolve(value).or(default)
    }
}
