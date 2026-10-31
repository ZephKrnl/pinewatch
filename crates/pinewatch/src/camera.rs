//! A development fly camera (not the game's camera, which arrives with the player in M4).
//!
//! WASD moves, Q/E go down/up, holding the right mouse button looks around, Shift moves faster.

use bevy::camera::{PerspectiveProjection, Projection};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use crate::terrain::LoadedTerrain;

#[derive(Component, Debug, Clone, Copy)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera).add_systems(Update, fly);
    }
}

fn spawn_camera(mut commands: Commands, terrain: Res<LoadedTerrain>) {
    let (center, width) = terrain.view_target();
    // South of the landscape's center (Bevy +Z is Unity -Z), high enough to see the whole map.
    let eye = center + Vec3::new(0.0, width * 0.45, width * 0.75);
    let transform = Transform::from_translation(eye).looking_at(center, Vec3::Y);
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection { far: width * 4.0 + 4096.0, ..default() }),
        transform,
        FlyCamera { yaw, pitch },
        AmbientLight { color: Color::WHITE, brightness: 400.0, affects_lightmapped_meshes: true },
    ));
}

fn fly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut cameras: Query<(&mut Transform, &mut FlyCamera)>,
) {
    for (mut transform, mut camera) in &mut cameras {
        if buttons.pressed(MouseButton::Right) {
            camera.yaw -= motion.delta.x * 0.003;
            camera.pitch = (camera.pitch - motion.delta.y * 0.003).clamp(-1.54, 1.54);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
        }

        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        let mut direction = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            direction += forward;
        }
        if keys.pressed(KeyCode::KeyS) {
            direction -= forward;
        }
        if keys.pressed(KeyCode::KeyD) {
            direction += right;
        }
        if keys.pressed(KeyCode::KeyA) {
            direction -= right;
        }
        if keys.pressed(KeyCode::KeyE) {
            direction += Vec3::Y;
        }
        if keys.pressed(KeyCode::KeyQ) {
            direction -= Vec3::Y;
        }
        let fast = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let speed = if fast { 600.0 } else { 120.0 };
        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * speed * time.delta_secs();
        }
    }
}
