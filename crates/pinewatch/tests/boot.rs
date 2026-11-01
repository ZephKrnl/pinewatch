use bevy::math::Vec3;
use bevy::mesh::{Mesh, VertexAttributeValues};
use pinewatch::cli::{Cli, parse_args};
use pinewatch::space::unity_to_bevy;
use pinewatch::terrain::build_tile_mesh;
use pinewatch_level::{LandscapeCoord, LandscapeTile};

fn positions(mesh: &Mesh) -> &[[f32; 3]] {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(p)) => p,
        other => panic!("unexpected positions: {other:?}"),
    }
}

fn normals(mesh: &Mesh) -> &[[f32; 3]] {
    match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(n)) => n,
        other => panic!("unexpected normals: {other:?}"),
    }
}

#[test]
fn unity_to_bevy_negates_z() {
    assert_eq!(unity_to_bevy([1.0, 2.0, 3.0]), Vec3::new(1.0, 2.0, -3.0));
}

#[test]
fn flat_half_height_tile_sits_at_zero_and_spans_one_tile() {
    let tile = LandscapeTile::new(LandscapeCoord { x: 1, y: -2 });
    let mesh = build_tile_mesh(&tile);
    let p = positions(&mesh);
    assert_eq!(p.len(), 257 * 257);
    assert!(p.iter().all(|v| v[1] == 0.0));
    // Unity (1024, 0, -2048) for row 0 / column 0, Unity (2048, 0, -1024) for row 256 / column 256.
    assert_eq!(p[0], [1024.0, 0.0, 2048.0]);
    assert_eq!(p[257 * 257 - 1], [2048.0, 0.0, 1024.0]);
    assert_eq!(mesh.indices().unwrap().len(), 256 * 256 * 6);
}

#[test]
fn heightmap_rows_follow_unity_z_and_columns_follow_unity_x() {
    let mut tile = LandscapeTile::new(LandscapeCoord { x: 0, y: 0 });
    tile.heightmap[3][5] = 1.0;
    tile.heightmap[4][5] = 0.0;
    let mesh = build_tile_mesh(&tile);
    let p = positions(&mesh);
    assert_eq!(p[3 * 257 + 5], [20.0, 1024.0, -12.0]);
    assert_eq!(p[4 * 257 + 5], [20.0, -1024.0, -16.0]);
}

#[test]
fn flat_tile_normals_point_up() {
    let mesh = build_tile_mesh(&LandscapeTile::new(LandscapeCoord { x: 0, y: 0 }));
    assert!(normals(&mesh).iter().all(|n| (n[1] - 1.0).abs() < 1e-5 && n[0].abs() < 1e-5 && n[2].abs() < 1e-5));
}

fn args(list: &[&str]) -> Result<Cli, String> {
    parse_args(list.iter().map(std::ffi::OsString::from))
}

#[test]
fn cli_defaults() {
    let cli = args(&["pinewatch"]).unwrap();
    assert_eq!(cli, Cli { unturned: None, map: "PEI".into(), screenshot: None, help: false });
}

#[test]
fn cli_options() {
    let cli = args(&["pinewatch", "--map", "Washington", "--unturned", "D:/Games/U", "--screenshot", "a.png"]).unwrap();
    assert_eq!(cli.map, "Washington");
    assert_eq!(cli.unturned, Some("D:/Games/U".into()));
    assert_eq!(cli.screenshot, Some("a.png".into()));
}

#[test]
fn cli_rejects_unknown_options_and_missing_values() {
    assert!(args(&["pinewatch", "--frobnicate"]).unwrap_err().contains("--frobnicate"));
    assert!(args(&["pinewatch", "--map"]).unwrap_err().contains("--map"));
}

#[test]
fn cli_requires_a_png_screenshot() {
    assert!(args(&["pinewatch", "--screenshot", "shot.jpg"]).unwrap_err().contains(".png"));
    assert!(args(&["pinewatch", "--screenshot", "shot"]).unwrap_err().contains(".png"));
    assert_eq!(args(&["pinewatch", "--screenshot", "Shot.PNG"]).unwrap().screenshot, Some("Shot.PNG".into()));
}

/// A folder under `%TEMP%`, removed when dropped.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("pinewatch-boot-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn screenshot_written_requires_a_new_or_changed_file() {
    use pinewatch::screenshot::{modified_time, was_written};
    let dir = TempDir::new("screenshot");
    let path = dir.0.join("shot.png");
    let _ = std::fs::remove_file(&path);

    let before = modified_time(&path);
    assert_eq!(before, None);
    assert!(!was_written(&path, before), "a missing file is not written");
    std::fs::write(&path, b"1").unwrap();
    assert!(was_written(&path, before), "a new file is written");

    let before = modified_time(&path);
    assert!(!was_written(&path, before), "an untouched old file is not written");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&path, b"2").unwrap();
    assert!(was_written(&path, before), "a rewritten file is written");
}

#[test]
fn screenshot_paths_inside_the_install_are_detected() {
    use pinewatch::screenshot::inside_install;
    let dir = TempDir::new("install");
    let root = dir.0.clone();
    std::fs::create_dir_all(root.join("Maps")).unwrap();
    let name = root.file_name().unwrap().to_owned();
    assert!(inside_install(&root.join("a.png"), &root).unwrap());
    assert!(inside_install(&root.join("Maps").join("not-yet").join("a.png"), &root).unwrap());
    assert!(inside_install(&root.join("..").join(&name).join("a.png"), &root).unwrap());
    assert!(!inside_install(&root.with_file_name("pinewatch-elsewhere").join("a.png"), &root).unwrap());
    assert!(!inside_install(&root.join("..").join("a.png"), &root).unwrap());
}
