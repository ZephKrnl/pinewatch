//! Terrain meshes built from the original landscape tiles.
//!
//! Positions and heights follow Unity's `TerrainData` exactly: tile origin `(x * 1024, -1024, y * 1024)`, size
//! `(1024, 2048, 1024)`, heightmap rows along world z and columns along world x.
//!
//! TEMPORARY (until M2): colors come from a fixed placeholder palette weighted by the splatmap, because the
//! original terrain materials live in Unity bundles that M2's asset processor converts.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use pinewatch_level::{
    HEIGHTMAP_RESOLUTION, LandscapeTile, SPLATMAP_LAYERS, SPLATMAP_RESOLUTION, TILE_HEIGHT, TILE_SIZE,
};

use crate::space::unity_to_bevy;

/// TEMPORARY placeholder color per splat layer (linear RGB), until M2 brings the original materials.
pub const PLACEHOLDER_LAYER_COLORS: [[f32; 3]; SPLATMAP_LAYERS] = [
    [0.20, 0.36, 0.12],
    [0.32, 0.26, 0.16],
    [0.45, 0.43, 0.40],
    [0.62, 0.56, 0.38],
    [0.15, 0.28, 0.10],
    [0.40, 0.33, 0.24],
    [0.80, 0.82, 0.85],
    [0.28, 0.30, 0.20],
];

const CELL: f32 = TILE_SIZE / (HEIGHTMAP_RESOLUTION - 1) as f32;

/// Unity-space position of heightmap sample `(row, column)`.
pub fn tile_vertex_unity(tile: &LandscapeTile, row: usize, column: usize) -> [f32; 3] {
    let origin_x = tile.coord.x as f32 * TILE_SIZE;
    let origin_z = tile.coord.y as f32 * TILE_SIZE;
    [
        origin_x + column as f32 * CELL,
        -TILE_HEIGHT / 2.0 + tile.heightmap[row][column] * TILE_HEIGHT,
        origin_z + row as f32 * CELL,
    ]
}

fn placeholder_color(tile: &LandscapeTile, row: usize, column: usize) -> [f32; 4] {
    let weights = &tile.splatmap[row.min(SPLATMAP_RESOLUTION - 1)][column.min(SPLATMAP_RESOLUTION - 1)];
    let total: f32 = weights.iter().sum();
    let mut rgb = [0.0f32; 3];
    for (weight, color) in weights.iter().zip(PLACEHOLDER_LAYER_COLORS) {
        for (channel, value) in rgb.iter_mut().zip(color) {
            *channel += weight * value;
        }
    }
    if total > 0.0 {
        rgb.iter_mut().for_each(|c| *c /= total);
    }
    [rgb[0], rgb[1], rgb[2], 1.0]
}

/// One tile as a triangle mesh in Bevy space, facing up (+Y).
pub fn build_tile_mesh(tile: &LandscapeTile) -> Mesh {
    let n = HEIGHTMAP_RESOLUTION;
    let mut positions = Vec::with_capacity(n * n);
    let mut colors = Vec::with_capacity(n * n);
    for row in 0..n {
        for column in 0..n {
            positions.push(unity_to_bevy(tile_vertex_unity(tile, row, column)).to_array());
            colors.push(placeholder_color(tile, row, column));
        }
    }

    let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for row in 0..n - 1 {
        for column in 0..n - 1 {
            let a = (row * n + column) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            // Counter-clockwise seen from above once Z is flipped into Bevy space.
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}

/// The loaded map's terrain.
#[derive(Resource)]
pub struct LoadedTerrain {
    pub map: String,
    pub tiles: Vec<LandscapeTile>,
}

impl LoadedTerrain {
    /// Bevy-space center of the landscape (at its height there) and the landscape's width in world units.
    pub fn view_target(&self) -> (Vec3, f32) {
        let min_x = self.tiles.iter().map(|t| t.coord.x).min().unwrap_or(0);
        let max_x = self.tiles.iter().map(|t| t.coord.x).max().unwrap_or(0);
        let min_y = self.tiles.iter().map(|t| t.coord.y).min().unwrap_or(0);
        let max_y = self.tiles.iter().map(|t| t.coord.y).max().unwrap_or(0);
        let center_x = (min_x + max_x + 1) as f32 * TILE_SIZE / 2.0;
        let center_z = (min_y + max_y + 1) as f32 * TILE_SIZE / 2.0;
        let height = self
            .tiles
            .iter()
            .find(|t| {
                let x0 = t.coord.x as f32 * TILE_SIZE;
                let z0 = t.coord.y as f32 * TILE_SIZE;
                (x0..x0 + TILE_SIZE).contains(&center_x) && (z0..z0 + TILE_SIZE).contains(&center_z)
            })
            .map(|t| {
                let column = (((center_x - t.coord.x as f32 * TILE_SIZE) / CELL) as usize).min(HEIGHTMAP_RESOLUTION - 1);
                let row = (((center_z - t.coord.y as f32 * TILE_SIZE) / CELL) as usize).min(HEIGHTMAP_RESOLUTION - 1);
                tile_vertex_unity(t, row, column)[1]
            })
            .unwrap_or(0.0);
        let width = ((max_x - min_x + 1).max(max_y - min_y + 1)) as f32 * TILE_SIZE;
        (unity_to_bevy([center_x, height, center_z]), width)
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_terrain);
    }
}

fn spawn_terrain(
    mut commands: Commands,
    terrain: Res<LoadedTerrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // TEMPORARY until M2: one plain material; vertex colors carry the placeholder layer palette.
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });
    for tile in &terrain.tiles {
        commands.spawn((
            Mesh3d(meshes.add(build_tile_mesh(tile))),
            MeshMaterial3d(material.clone()),
            Transform::IDENTITY,
            Name::new(format!("Terrain {}", tile.coord)),
        ));
    }
    commands.spawn((
        DirectionalLight { illuminance: bevy::light::light_consts::lux::AMBIENT_DAYLIGHT, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, 0.6, -0.9, 0.0)),
    ));
}
