//! Ports of `Landscape.read`, `LandscapeTile.read`, `readHeightmap`, `readSplatmap`, `ReadHoles`,
//! `LandscapeCoord.read`, `AssetReference.read` and the item loop of `LevelHierarchy.read`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};
use pinewatch_level::{LandscapeCoord, LandscapeTile, default_material, read_landscape};
use pinewatch_mono::{ExceptionKind, Guid, NetString};
use sha1::{Digest, Sha1};

const LANDSCAPE: &str = "SDG.Framework.Landscapes.Landscape, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";
const HEIGHTMAP_BYTES: usize = 257 * 257 * 2;
const SPLATMAP_BYTES: usize = 256 * 256 * 8;
const GUID_A: &str = "53d3ea4a555c44d1824b495c2b337aec";
const GUID_B: &str = "498ca625072d443a876b2a4f11896019";

fn temp_install() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("pinewatch-level-test-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Bundles")).unwrap();
    fs::create_dir_all(dir.join("Maps").join("T").join("Landscape").join("Heightmaps")).unwrap();
    fs::create_dir_all(dir.join("Maps").join("T").join("Landscape").join("Splatmaps")).unwrap();
    fs::create_dir_all(dir.join("Maps").join("T").join("Landscape").join("Holes")).unwrap();
    dir
}

fn tile_entry(x: i32, y: i32, materials: &str) -> String {
    format!("{{ \"Coord\" {{ \"X\" \"{x}\" \"Y\" \"{y}\" }} \"Materials\" [ {materials} ] }}")
}

fn landscape_item(instance_id: &str, tiles: &[String]) -> String {
    format!(
        "{{ \"Type\" \"{LANDSCAPE}\" {instance_id} \"Item\" {{ \"Tiles\" [ {} ] }} }}",
        tiles.join(" ")
    )
}

fn write_map(dir: &Path, items: &[String]) {
    let text = format!("\"Available_Instance_ID\" \"5\"\n\"Items\"\n[\n{}\n]\n", items.join("\n"));
    fs::write(dir.join("Maps/T/Level.hierarchy"), text).unwrap();
}

fn write_tile_files(dir: &Path, x: i32, y: i32, heightmap: &[u8], splatmap: &[u8]) {
    fs::write(dir.join(format!("Maps/T/Landscape/Heightmaps/Tile_{x}_{y}_Source.heightmap")), heightmap).unwrap();
    fs::write(dir.join(format!("Maps/T/Landscape/Splatmaps/Tile_{x}_{y}_Source.splatmap")), splatmap).unwrap();
}

fn one_tile(dir: &Path, heightmap: &[u8], splatmap: &[u8]) -> (Vec<LandscapeTile>, MemoryLog) {
    write_map(dir, &[landscape_item("\"Instance_ID\" \"2\"", &[tile_entry(0, 0, "")])]);
    write_tile_files(dir, 0, 0, heightmap, splatmap);
    let install = InstallRoot::open(dir).unwrap();
    let mut log = MemoryLog::default();
    let tiles = read_landscape(&install, "T", &mut log).unwrap().unwrap();
    (tiles, log)
}

#[test]
fn full_heightmap_decodes_big_endian() {
    let dir = temp_install();
    let heightmap: Vec<u8> = [0x80u8, 0x00].iter().copied().cycle().take(HEIGHTMAP_BYTES).collect();
    let (tiles, _) = one_tile(&dir, &heightmap, &vec![0; SPLATMAP_BYTES]);
    assert_eq!(tiles.len(), 1);
    assert_eq!(tiles[0].heightmap[0][0], 0x8000 as f32 / 65535.0);
    assert_eq!(tiles[0].heightmap[256][256], 0x8000 as f32 / 65535.0);
}

#[test]
fn heightmap_one_byte_short_makes_last_sample_full_height() {
    let dir = temp_install();
    let (tiles, _) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES - 1], &vec![0; SPLATMAP_BYTES]);
    assert_eq!(tiles[0].heightmap[256][255], 0.0);
    assert_eq!(tiles[0].heightmap[256][256], 1.0);
}

#[test]
fn empty_splatmap_reads_every_layer_as_full() {
    let dir = temp_install();
    let (tiles, _) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &[]);
    assert!(tiles[0].splatmap.iter().flatten().flatten().all(|&w| w == 1.0));
}

#[test]
fn splatmap_layers_are_innermost() {
    let dir = temp_install();
    let mut splatmap = vec![0u8; SPLATMAP_BYTES];
    splatmap[3] = 255; // row 0, column 0, layer 3
    splatmap[8 + 1] = 51; // row 0, column 1, layer 1
    let (tiles, _) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &splatmap);
    assert_eq!(tiles[0].splatmap[0][0][3], 1.0);
    assert_eq!(tiles[0].splatmap[0][1][1], 51.0 / 255.0);
    assert_eq!(tiles[0].splatmap[0][0][0], 0.0);
}

#[test]
fn holes_bits_least_significant_first_and_one_means_solid() {
    let dir = temp_install();
    let mut holes = vec![1u8]; // version
    holes.push(0b0000_0001);
    holes.extend(std::iter::repeat_n(0xFF, 256 * 32 - 1));
    fs::write(dir.join("Maps/T/Landscape/Holes/Tile_0_0.bin"), &holes).unwrap();
    let (tiles, _) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    assert!(tiles[0].has_any_holes_data);
    assert!(tiles[0].holes[0][0]);
    assert!(!tiles[0].holes[0][1]);
    assert!(!tiles[0].holes[0][7]);
    assert!(tiles[0].holes[0][8]);
    assert!(tiles[0].holes[255][255]);
}

#[test]
fn without_holes_file_every_cell_is_solid() {
    let dir = temp_install();
    let (tiles, log) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    assert!(!tiles[0].has_any_holes_data);
    assert!(tiles[0].holes.iter().flatten().all(|&solid| solid));
    assert_eq!(tiles[0].hashes.holes, None);
    assert!(log.entries.iter().all(|(level, _)| *level == LogLevel::Info));
}

#[test]
fn missing_heightmap_logs_the_exact_warning_and_keeps_default_heights() {
    let dir = temp_install();
    write_map(&dir, &[landscape_item("", &[tile_entry(0, 0, "")])]);
    fs::write(dir.join("Maps/T/Landscape/Splatmaps/Tile_0_0_Source.splatmap"), vec![0u8; SPLATMAP_BYTES]).unwrap();
    let install = InstallRoot::open(&dir).unwrap();
    let mut log = MemoryLog::default();
    let tiles = read_landscape(&install, "T", &mut log).unwrap().unwrap();
    let expected = format!(
        "LandscapeTile missing heightmap file: {}\\Maps\\T/Landscape/Heightmaps/Tile_0_0_Source.heightmap",
        dir.to_string_lossy()
    );
    assert!(log.entries.contains(&(LogLevel::Warn, expected)), "{:?}", log.entries);
    assert_eq!(tiles[0].heightmap[100][100], 0.5);
    assert_eq!(tiles[0].hashes.heightmap, None);
}

#[test]
fn hashes_cover_exactly_the_bytes_read() {
    let dir = temp_install();
    let mut heightmap = vec![7u8; HEIGHTMAP_BYTES];
    heightmap.extend_from_slice(b"trailing bytes are not read");
    let (tiles, _) = one_tile(&dir, &heightmap, &vec![0; SPLATMAP_BYTES]);
    let expected: [u8; 20] = Sha1::digest(&heightmap[..HEIGHTMAP_BYTES]).into();
    assert_eq!(tiles[0].hashes.heightmap, Some(expected));
}

#[test]
fn materials_from_strings_and_tables_with_defaults_for_the_rest() {
    let dir = temp_install();
    let materials = format!("\"{GUID_A}\" {{ \"GUID\" \"{GUID_B}\" }}");
    write_map(&dir, &[landscape_item("", &[tile_entry(0, 0, &materials)])]);
    write_tile_files(&dir, 0, 0, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    let install = InstallRoot::open(&dir).unwrap();
    let tiles = read_landscape(&install, "T", &mut MemoryLog::default()).unwrap().unwrap();
    let m = &tiles[0].materials;
    assert_eq!(m.len(), 8);
    assert_eq!(m[0].to_string_n(), GUID_A);
    assert_eq!(m[1].to_string_n(), GUID_B);
    assert!(m[2..].iter().all(|g| *g == Guid::EMPTY));
}

#[test]
fn default_first_material_is_fallback_green() {
    assert_eq!(default_material().to_string_n(), "498ca625072d443a876b2a4f11896018");
    let dir = temp_install();
    let (tiles, _) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    assert_eq!(tiles[0].materials[0], default_material());
}

#[test]
fn more_than_eight_materials_throws() {
    let dir = temp_install();
    let materials = vec![format!("\"{GUID_A}\""); 9].join(" ");
    write_map(&dir, &[landscape_item("", &[tile_entry(0, 0, &materials)])]);
    write_tile_files(&dir, 0, 0, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    let install = InstallRoot::open(&dir).unwrap();
    let err = read_landscape(&install, "T", &mut MemoryLog::default()).unwrap_err();
    assert_eq!(err.kind, ExceptionKind::ArgumentOutOfRange);
}

#[test]
fn loading_message_counts_tiles() {
    let dir = temp_install();
    let (_, log) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    assert_eq!(log.entries, vec![(LogLevel::Info, "Loading 1 landscape tiles".to_string())]);
}

#[test]
fn map_without_a_landscape_item_has_no_landscape() {
    let dir = temp_install();
    write_map(&dir, &[]);
    let install = InstallRoot::open(&dir).unwrap();
    assert!(read_landscape(&install, "T", &mut MemoryLog::default()).unwrap().is_none());
}

#[test]
fn map_without_a_hierarchy_has_no_landscape() {
    let dir = temp_install();
    let install = InstallRoot::open(&dir).unwrap();
    assert!(read_landscape(&install, "T", &mut MemoryLog::default()).unwrap().is_none());
}

#[test]
fn second_landscape_is_ignored_with_a_warning() {
    let dir = temp_install();
    write_map(
        &dir,
        &[
            landscape_item("\"Instance_ID\" \"2\"", &[tile_entry(0, 0, "")]),
            landscape_item("", &[tile_entry(1, 0, ""), tile_entry(2, 0, "")]),
        ],
    );
    write_tile_files(&dir, 0, 0, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    let install = InstallRoot::open(&dir).unwrap();
    let mut log = MemoryLog::default();
    let tiles = read_landscape(&install, "T", &mut log).unwrap().unwrap();
    assert_eq!(tiles.len(), 1);
    // The second item has no Instance_ID, so it gets the next free ID: Available_Instance_ID was 5.
    assert!(log.entries.contains(&(
        LogLevel::Warn,
        "Level contains multiple Landscapes. Ignoring 2 tile(s) with instance ID: 5".to_string()
    )));
}

#[test]
fn duplicate_coord_is_reported_and_skipped() {
    let dir = temp_install();
    write_map(&dir, &[landscape_item("", &[tile_entry(0, 0, ""), tile_entry(0, 0, "")])]);
    write_tile_files(&dir, 0, 0, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    let install = InstallRoot::open(&dir).unwrap();
    let mut log = MemoryLog::default();
    let tiles = read_landscape(&install, "T", &mut log).unwrap().unwrap();
    assert_eq!(tiles.len(), 1);
    assert!(log.entries.contains(&(LogLevel::Error, "Duplicate landscape coord read: (0, 0)".to_string())));
}

#[test]
fn coord_without_a_table_throws_null_reference() {
    let dir = temp_install();
    write_map(&dir, &[landscape_item("", &["{ \"Materials\" [ ] }".to_string()])]);
    let install = InstallRoot::open(&dir).unwrap();
    let err = read_landscape(&install, "T", &mut MemoryLog::default()).unwrap_err();
    assert_eq!(err.kind, ExceptionKind::NullReference);
}

#[test]
fn coord_display_matches_csharp_to_string() {
    assert_eq!(LandscapeCoord { x: -1, y: 2 }.to_string(), "(-1, 2)");
    let _ = NetString::new();
}
