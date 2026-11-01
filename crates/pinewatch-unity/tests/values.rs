//! Unity's value types: field layout, constructors and `Equals`, as the IL in the installed game's
//! `UnityEngine.CoreModule.dll` defines them.

use pinewatch_unity::{Color, Color32, Vector2, Vector3};

#[test]
fn equality_is_exact_per_component() {
    // `Vector2.Equals` and `Vector3.Equals` compare each component with `==` (`bne.un`, then `ceq`).
    assert_ne!(Vector3::new(0.1, 0.0, 0.0), Vector3::new(0.1000001, 0.0, 0.0));
    assert_eq!(Vector3::new(-0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
    let nan = Vector2::new(f32::NAN, 0.0);
    assert_ne!(nan, nan);
    let nan = Vector3::new(0.0, 0.0, f32::NAN);
    assert_ne!(nan, nan);
}

#[test]
fn color_equality_counts_nan_as_equal() {
    // `Color.Equals` compares each component with `Single.Equals`: `==`, or both NaN (any sign or payload).
    assert_eq!(Color::new(0.0, 0.0, 0.0, f32::NAN), Color::new(0.0, 0.0, 0.0, f32::NAN));
    assert_eq!(Color::new(0.0, f32::NAN, 0.0, 1.0), Color::new(0.0, -f32::NAN, 0.0, 1.0));
    assert_ne!(Color::new(0.0, 0.0, 0.0, f32::NAN), Color::new(0.0, 0.0, 0.0, 0.0));
    assert_ne!(Color::new(0.1, 0.0, 0.0, 1.0), Color::new(0.1000001, 0.0, 0.0, 1.0));
    assert_eq!(Color::new(-0.0, 0.0, 0.0, 1.0), Color::new(0.0, 0.0, 0.0, 1.0));
}

#[test]
fn three_argument_color_constructor_sets_alpha_to_one() {
    // `new Color(r, g, b)` stores 1065353216 (`1.0f`) in `a`.
    assert_eq!(Color::rgb(0.25, 0.5, 0.75), Color::new(0.25, 0.5, 0.75, 1.0));
    assert_eq!(Color::rgb(0.0, 0.0, 0.0).a.to_bits(), 0x3F80_0000);
}

#[test]
fn color32_compares_its_bytes() {
    assert_eq!(Color32::new(1, 2, 3, 4), Color32 { r: 1, g: 2, b: 3, a: 4 });
    assert_ne!(Color32::new(1, 2, 3, 4), Color32::new(1, 2, 3, 5));
}

/// `implicit operator Color(Color32 c)`: each channel through `conv.r4`, then divided by `255f`. The expected bits
/// were computed apart from Rust (`n / 255` rounded to single precision).
#[test]
fn color32_converts_to_color_by_dividing_by_255() {
    let color = Color::from(Color32::new(0, 128, 255, 1));
    assert_eq!(0.0f32.to_bits(), color.r.to_bits());
    assert_eq!(0x3F00_8081, color.g.to_bits());
    assert_eq!(1.0f32.to_bits(), color.b.to_bits());
    assert_eq!(0x3B80_8081, color.a.to_bits());
    assert_eq!(0x3F7E_FEFF, Color::from(Color32::new(254, 0, 0, 0)).r.to_bits());
}

#[test]
fn layout_matches_unity() {
    assert_eq!(std::mem::size_of::<Vector2>(), 8);
    assert_eq!(std::mem::size_of::<Vector3>(), 12);
    assert_eq!(std::mem::size_of::<Color>(), 16);
    assert_eq!(std::mem::size_of::<Color32>(), 4);
}

#[test]
fn default_is_all_zeros() {
    assert_eq!(Vector2::default(), Vector2::new(0.0, 0.0));
    assert_eq!(Vector3::default(), Vector3::new(0.0, 0.0, 0.0));
    assert_eq!(Color::default(), Color::new(0.0, 0.0, 0.0, 0.0));
    assert_eq!(Color32::default(), Color32::new(0, 0, 0, 0));
}
