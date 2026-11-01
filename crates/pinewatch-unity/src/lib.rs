//! Unity value types exactly as the original game stores them.
//!
//! Each struct mirrors the one of the same name in the installed game's `UnityEngine.CoreModule.dll` (Unity
//! 2022.3.62f3): the same public fields in the same order, so the same size. The constructors, `Default`,
//! `PartialEq` and the `Color32` to `Color` conversion follow that assembly's IL:
//!
//! - `Default` is C#'s `default(T)`: every field zero.
//! - `PartialEq` is the type's `Equals(T)`. `Vector2` and `Vector3` compare each component with `==`, so a NaN
//!   component makes a value unequal to itself. `Color` compares each component with `Single.Equals`, which also
//!   counts two NaNs as equal. `Color32` doesn't override `Equals`, so `ValueType.Equals` compares its bytes.
//!
//! Unity's `==` operators on `Vector2`, `Vector3` and `Color` are a different, approximate comparison, and aren't
//! ported. `Color32`'s private `rgba` field, an `int` laid over the four bytes, isn't ported either: it only makes
//! Unity's struct 4-byte aligned, and its constructor's `rgba = 0` is overwritten by the four byte stores.

/// `UnityEngine.Vector2`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    /// `new Vector2(x, y)`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// `UnityEngine.Vector3`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    /// `new Vector3(x, y, z)`.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// `UnityEngine.Color`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// `new Color(r, g, b, a)`.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// `new Color(r, g, b)`: `a` is `1.0`.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

impl From<Color32> for Color {
    /// `implicit operator Color(Color32 c)`: each channel through `conv.r4`, then divided by `255f`. One division of
    /// two exact values rounds the same whether Mono computes it in single or double precision.
    fn from(c: Color32) -> Self {
        Self::new(f32::from(c.r) / 255.0, f32::from(c.g) / 255.0, f32::from(c.b) / 255.0, f32::from(c.a) / 255.0)
    }
}

impl PartialEq for Color {
    /// `Color.Equals(Color)`: `Single.Equals` on `r`, `g`, `b` and `a`.
    fn eq(&self, other: &Self) -> bool {
        single_equals(self.r, other.r)
            && single_equals(self.g, other.g)
            && single_equals(self.b, other.b)
            && single_equals(self.a, other.a)
    }
}

/// `value.Equals(obj)` for `float`: `obj == value`, or both are NaN.
fn single_equals(value: f32, obj: f32) -> bool {
    obj == value || (obj.is_nan() && value.is_nan())
}

/// `UnityEngine.Color32`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Color32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color32 {
    /// `new Color32(r, g, b, a)`.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}
