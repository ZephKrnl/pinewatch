//! Ports of `Landscape.read`, `LandscapeTile.read` (with its asset checks and holiday redirects),
//! `updatePrototypes`' asset checks, `readHeightmap`, `readSplatmap`, `ReadHoles`, `LandscapeCoord.read`,
//! `AssetReference.read` and the item loop of `LevelHierarchy.read`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{AssetLoader, ClientAssetIntegrity, ENPCHoliday, load_client_assets};
use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};
use pinewatch_level::{LandscapeCoord, LandscapeTile, TileAssets, default_material, read_landscape};
use pinewatch_mono::{ExceptionKind, Guid, MonoException, NetString};
use sha1::{Digest, Sha1};

const LANDSCAPE: &str = "SDG.Framework.Landscapes.Landscape, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";
const HEIGHTMAP_BYTES: usize = 257 * 257 * 2;
const SPLATMAP_BYTES: usize = 256 * 256 * 8;
const GUID_A: &str = "53d3ea4a555c44d1824b495c2b337aec";
const GUID_B: &str = "498ca625072d443a876b2a4f11896019";
const GUID_C: &str = "6b2a3d4c5e6f708192a3b4c5d6e7f809";
const REDIRECTOR: &str = "7b2a3d4c5e6f708192a3b4c5d6e7f809";
const DEFAULT_MATERIAL: &str = "498ca625072d443a876b2a4f11896018";
const CORE_CONFIG: &str = "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 6";

/// A temporary install under `%TEMP%`, removed when dropped.
struct TempInstall(PathBuf);

impl std::ops::Deref for TempInstall {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TempInstall {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_install() -> TempInstall {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("pinewatch-level-test-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Bundles")).unwrap();
    fs::create_dir_all(dir.join("Maps").join("T").join("Landscape").join("Heightmaps")).unwrap();
    fs::create_dir_all(dir.join("Maps").join("T").join("Landscape").join("Splatmaps")).unwrap();
    fs::create_dir_all(dir.join("Maps").join("T").join("Landscape").join("Holes")).unwrap();
    TempInstall(dir)
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

/// A landscape material with this GUID and these extra `Asset` lines.
fn material(guid: &str, extra: &str) -> String {
    format!(
        "Metadata\n{{\n\tGUID {guid}\n\tType \"SDG.Unturned.LandscapeMaterialAsset, Assembly-CSharp\"\n}}\n\
         Asset\n{{\n{extra}\n}}"
    )
}

/// The assets of `dir`: the core master bundle and `files` under `Bundles`, loaded as the client loads them.
fn load_assets(dir: &Path, files: &[(&str, String)]) -> AssetLoader {
    fs::write(dir.join("Bundles/MasterBundle.dat"), CORE_CONFIG).unwrap();
    fs::write(dir.join("Bundles/core.masterbundle"), "").unwrap();
    for (file, contents) in files {
        let path = dir.join("Bundles").join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(dir).unwrap();
    load_client_assets(&install, "English", &mut MemoryLog::default()).unwrap()
}

/// Assets with only the default material, which a tile keeps in its first slot unless the map names another.
fn default_assets(dir: &Path) -> AssetLoader {
    load_assets(dir, &[("Default/Default.asset", material(DEFAULT_MATERIAL, ""))])
}

/// `read_landscape` for map `T` with `assets`, a fresh integrity state, and holiday redirects on for `holiday`
/// (`None`: off).
fn read_with_holiday(
    dir: &Path,
    assets: &AssetLoader,
    holiday: Option<ENPCHoliday>,
    log: &mut MemoryLog,
) -> Result<Option<Vec<LandscapeTile>>, MonoException> {
    let install = InstallRoot::open(dir).unwrap();
    let mut integrity = ClientAssetIntegrity::default();
    let mut context = TileAssets::new(assets, &mut integrity);
    if let Some(holiday) = holiday {
        context.use_holiday_redirects = true;
        context.active_holiday = holiday;
    }
    read_landscape(&install, "T", &mut context, log)
}

fn read(dir: &Path, assets: &AssetLoader, log: &mut MemoryLog) -> Result<Option<Vec<LandscapeTile>>, MonoException> {
    read_with_holiday(dir, assets, None, log)
}

fn one_tile(dir: &Path, heightmap: &[u8], splatmap: &[u8]) -> (Vec<LandscapeTile>, MemoryLog) {
    write_map(dir, &[landscape_item("\"Instance_ID\" \"2\"", &[tile_entry(0, 0, "")])]);
    write_tile_files(dir, 0, 0, heightmap, splatmap);
    let assets = default_assets(dir);
    let mut log = MemoryLog::default();
    let tiles = read(dir, &assets, &mut log).unwrap().unwrap();
    (tiles, log)
}

/// A map with one tile at (`x`, `y`) naming `materials`, with full heightmap and splatmap files.
fn one_tile_with_materials(dir: &Path, x: i32, y: i32, materials: &str) {
    write_map(dir, &[landscape_item("", &[tile_entry(x, y, materials)])]);
    write_tile_files(dir, x, y, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
}

fn info(text: &str) -> (LogLevel, String) {
    (LogLevel::Info, text.to_string())
}

fn known_missing(context: &str, guid: &str) -> (LogLevel, String) {
    info(&format!("Context \"{context}\" known missing asset {guid}, server will not kick clients for this"))
}

fn missing(context: &str, guid: &str) -> (LogLevel, String) {
    (LogLevel::Warn, format!("Context \"{context}\" missing asset {guid}"))
}

fn guid(text: &str) -> Guid {
    Guid::try_parse(Some(&NetString::from(text))).unwrap()
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
    let mut log = MemoryLog::default();
    let tiles = read(&dir, &default_assets(&dir), &mut log).unwrap().unwrap();
    // Path.Combine joins with the platform separator: `\` on Windows (the reference platform), `/` on Linux.
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let expected = format!(
        "LandscapeTile missing heightmap file: {}{separator}Maps{separator}T/Landscape/Heightmaps/Tile_0_0_Source.heightmap",
        dir.to_string_lossy()
    );
    assert!(log.entries.contains(&(LogLevel::Warn, expected)), "{:?}", log.entries);
    assert_eq!(tiles[0].heightmap[100][100], 0.5);
    assert_eq!(tiles[0].hashes.heightmap, None);
}

/// V11: the root is `DirectoryInfo.FullName`, so a path with `..` appears absolute and normalized.
#[cfg(windows)]
#[test]
fn message_root_is_absolute_and_normalized() {
    let dir = temp_install();
    write_map(&dir, &[landscape_item("", &[tile_entry(0, 0, "")])]);
    fs::write(dir.join("Maps/T/Landscape/Splatmaps/Tile_0_0_Source.splatmap"), vec![0u8; SPLATMAP_BYTES]).unwrap();
    let assets = default_assets(&dir);
    let detour = dir.join("..").join(dir.file_name().unwrap());
    let install = InstallRoot::open(&detour).unwrap();
    let mut integrity = ClientAssetIntegrity::default();
    let mut log = MemoryLog::default();
    read_landscape(&install, "T", &mut TileAssets::new(&assets, &mut integrity), &mut log).unwrap();
    let expected = format!(
        "LandscapeTile missing heightmap file: {}\\Maps\\T/Landscape/Heightmaps/Tile_0_0_Source.heightmap",
        dir.to_string_lossy()
    );
    assert!(log.entries.contains(&(LogLevel::Warn, expected)), "{:?}", log.entries);
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
    one_tile_with_materials(&dir, 0, 0, &format!("\"{GUID_A}\" {{ \"GUID\" \"{GUID_B}\" }}"));
    let tiles = read(&dir, &load_assets(&dir, &[]), &mut MemoryLog::default()).unwrap().unwrap();
    let m = &tiles[0].materials;
    assert_eq!(m.len(), 8);
    assert_eq!(m[0].to_string_n(), GUID_A);
    assert_eq!(m[1].to_string_n(), GUID_B);
    assert!(m[2..].iter().all(|g| *g == Guid::EMPTY));
}

#[test]
fn default_first_material_is_fallback_green() {
    assert_eq!(default_material().to_string_n(), DEFAULT_MATERIAL);
    let dir = temp_install();
    let (tiles, _) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    assert_eq!(tiles[0].materials[0], default_material());
}

#[test]
fn more_than_eight_materials_throws() {
    let dir = temp_install();
    one_tile_with_materials(&dir, 0, 0, &vec![format!("\"{GUID_A}\""); 9].join(" "));
    let err = read(&dir, &default_assets(&dir), &mut MemoryLog::default()).unwrap_err();
    assert_eq!(err.kind, ExceptionKind::ArgumentOutOfRange);
    // `List<T>`'s indexer (V15, V7), with the platform's NewLine.
    let expected = format!(
        "Index was out of range. Must be non-negative and less than the size of the collection.{}Parameter name: index",
        pinewatch_mono::NEW_LINE
    );
    assert_eq!(err.message, expected);
}

#[test]
fn loading_message_counts_tiles() {
    let dir = temp_install();
    let (_, log) = one_tile(&dir, &vec![0; HEIGHTMAP_BYTES], &vec![0; SPLATMAP_BYTES]);
    assert_eq!(log.entries, vec![info("Loading 1 landscape tiles")]);
}

/// Materials the assets have pass both checks without a message.
#[test]
fn found_materials_log_nothing() {
    let dir = temp_install();
    one_tile_with_materials(&dir, 0, 0, &format!("\"{GUID_A}\" \"{GUID_B}\""));
    let assets = load_assets(&dir, &[("A/A.asset", material(GUID_A, "")), ("B/B.asset", material(GUID_B, ""))]);
    let mut log = MemoryLog::default();
    read(&dir, &assets, &mut log).unwrap();
    assert_eq!(log.entries, vec![info("Loading 1 landscape tiles")]);
}

/// A material the assets don't have is recorded as known missing while the tile reads (once per GUID), then
/// `updatePrototypes` queues each valid slot for the server's check, warning once per missing GUID.
#[test]
fn missing_materials_are_known_missing_then_queued() {
    let dir = temp_install();
    one_tile_with_materials(&dir, 3, -1, &format!("\"{GUID_A}\" \"{GUID_A}\" \"{GUID_B}\""));
    let mut log = MemoryLog::default();
    read(&dir, &load_assets(&dir, &[]), &mut log).unwrap();
    assert_eq!(log.entries, vec![
        info("Loading 1 landscape tiles"),
        known_missing("Landscape tile (x: 3 y: -1 layer: 0)", GUID_A),
        known_missing("Landscape tile (x: 3 y: -1 layer: 2)", GUID_B),
        missing("Landscape tile (x: 3 y: -1 layer: 0)", GUID_A),
        missing("Landscape tile (x: 3 y: -1 layer: 2)", GUID_B),
    ]);
}

/// The default material isn't in the map's list, so only `updatePrototypes` checks it.
#[test]
fn a_missing_default_material_is_only_queued() {
    let dir = temp_install();
    one_tile_with_materials(&dir, 0, 0, "");
    let mut log = MemoryLog::default();
    read(&dir, &load_assets(&dir, &[]), &mut log).unwrap();
    assert_eq!(log.entries, vec![
        info("Loading 1 landscape tiles"),
        missing("Landscape tile (x: 0 y: 0 layer: 0)", DEFAULT_MATERIAL),
    ]);
}

/// With holiday redirects on, a material with a valid redirect for the active holiday is replaced by it, before
/// the missing check; otherwise it stays.
#[test]
fn holiday_redirects_replace_materials() {
    let dir = temp_install();
    one_tile_with_materials(&dir, 0, 0, &format!("\"{GUID_A}\""));
    let redirects = format!("\tChristmas_Redirect {GUID_B}\n\tHalloween_Redirect {GUID_C}");
    let assets = load_assets(&dir, &[("A/A.asset", material(GUID_A, &redirects)), ("B/B.asset", material(GUID_B, ""))]);
    let first_material = |holiday: Option<ENPCHoliday>, log: &mut MemoryLog| {
        read_with_holiday(&dir, &assets, holiday, log).unwrap().unwrap()[0].materials[0]
    };

    let mut log = MemoryLog::default();
    assert_eq!(first_material(None, &mut log), guid(GUID_A));
    assert_eq!(first_material(Some(ENPCHoliday::CHRISTMAS), &mut log), guid(GUID_B));
    assert_eq!(first_material(Some(ENPCHoliday::APRIL_FOOLS), &mut log), guid(GUID_A));
    assert_eq!(first_material(Some(ENPCHoliday::VALENTINES), &mut log), guid(GUID_A));
    assert!(log.entries.iter().all(|(level, _)| *level == LogLevel::Info), "{:?}", log.entries);

    // Redirects off (say the level doesn't allow them) while Christmas is active: no swap.
    let install = InstallRoot::open(&dir).unwrap();
    let mut integrity = ClientAssetIntegrity::default();
    let mut context = TileAssets::new(&assets, &mut integrity);
    context.active_holiday = ENPCHoliday::CHRISTMAS;
    let tiles = read_landscape(&install, "T", &mut context, &mut log).unwrap().unwrap();
    assert_eq!(tiles[0].materials[0], guid(GUID_A));

    // The Halloween redirect names a material the assets don't have.
    let mut log = MemoryLog::default();
    assert_eq!(first_material(Some(ENPCHoliday::HALLOWEEN), &mut log), guid(GUID_C));
    assert_eq!(log.entries, vec![
        info("Loading 1 landscape tiles"),
        known_missing("Landscape tile (x: 0 y: 0 layer: 0)", GUID_C),
        missing("Landscape tile (x: 0 y: 0 layer: 0)", GUID_C),
    ]);
}

/// A slot keeps the GUID the map names even when it finds its material through a redirector; the holiday redirect
/// is the material's.
#[test]
fn materials_found_through_redirectors_keep_their_guid() {
    let dir = temp_install();
    one_tile_with_materials(&dir, 0, 0, &format!("\"{REDIRECTOR}\""));
    let assets = load_assets(&dir, &[
        ("A/A.asset", material(GUID_A, &format!("\tChristmas_Redirect {GUID_B}"))),
        ("B/B.asset", material(GUID_B, "")),
        ("R/R.dat", format!("GUID {REDIRECTOR}\nType Redirector\nTargetAsset {GUID_A}")),
    ]);
    let mut log = MemoryLog::default();
    let tiles = read(&dir, &assets, &mut log).unwrap().unwrap();
    assert_eq!(tiles[0].materials[0], guid(REDIRECTOR));
    assert_eq!(log.entries, vec![info("Loading 1 landscape tiles")]);
    let tiles = read_with_holiday(&dir, &assets, Some(ENPCHoliday::CHRISTMAS), &mut log).unwrap().unwrap();
    assert_eq!(tiles[0].materials[0], guid(GUID_B));
}

#[test]
fn map_without_a_landscape_item_has_no_landscape() {
    let dir = temp_install();
    write_map(&dir, &[]);
    assert!(read(&dir, &default_assets(&dir), &mut MemoryLog::default()).unwrap().is_none());
}

#[test]
fn map_without_a_hierarchy_has_no_landscape() {
    let dir = temp_install();
    assert!(read(&dir, &default_assets(&dir), &mut MemoryLog::default()).unwrap().is_none());
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
    let mut log = MemoryLog::default();
    let tiles = read(&dir, &default_assets(&dir), &mut log).unwrap().unwrap();
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
    let mut log = MemoryLog::default();
    let tiles = read(&dir, &default_assets(&dir), &mut log).unwrap().unwrap();
    assert_eq!(tiles.len(), 1);
    assert!(log.entries.contains(&(LogLevel::Error, "Duplicate landscape coord read: (0, 0)".to_string())));
}

#[test]
fn coord_without_a_table_throws_null_reference() {
    let dir = temp_install();
    write_map(&dir, &[landscape_item("", &["{ \"Materials\" [ ] }".to_string()])]);
    let err = read(&dir, &default_assets(&dir), &mut MemoryLog::default()).unwrap_err();
    assert_eq!(err.kind, ExceptionKind::NullReference);
}

#[test]
fn coord_display_matches_csharp_to_string() {
    assert_eq!(LandscapeCoord { x: -1, y: 2 }.to_string(), "(-1, 2)");
    let _ = NetString::new();
}
