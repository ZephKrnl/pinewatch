//! The one boundary between Unity space and Bevy space.
//!
//! The simulation keeps Unity's conventions (left-handed, Y up). Bevy is right-handed with Y up, so presenting a
//! Unity position only negates Z.

use bevy::math::Vec3;

/// Converts a Unity-space position to Bevy space.
pub fn unity_to_bevy(p: [f32; 3]) -> Vec3 {
    Vec3::new(p[0], p[1], -p[2])
}
