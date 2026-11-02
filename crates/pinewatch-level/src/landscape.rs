//! Landscape tiles: ports of `Landscape.read`, `LandscapeTile.read` (with its asset checks and holiday redirects),
//! `updatePrototypes`' asset checks, `readHeightmap`, `readSplatmap`, `ReadHoles`, `LandscapeCoord.read` and
//! `AssetReference.read` (Framework/Landscapes, Unturned/Bundles).

use std::fmt;
use std::io;

use pinewatch_assets::{AssetLoader, AssetReference, ClientAssetIntegrity, ENPCHoliday};
use pinewatch_io::mono_path::{self, PathStyle};
use pinewatch_io::{InstallRoot, LogLevel, LogSink};
use pinewatch_kvt::{KvtCursor, read_kvt_bytes};
use pinewatch_mono::{ExceptionKind, Guid, MonoException};
use sha1::{Digest, Sha1};

use crate::hierarchy::walk_hierarchy;

/// What reading tiles takes besides the map's files: the asset system and the state around it.
pub struct TileAssets<'a> {
    /// The loaded assets, for `AssetReference<LandscapeMaterialAsset>.Find()`.
    pub loader: &'a AssetLoader,
    /// `Level.shouldUseHolidayRedirects`: outside the editor, when the level allows holiday redirects and a
    /// holiday is active (V14).
    pub use_holiday_redirects: bool,
    /// `HolidayUtil.getActiveHoliday()`, which picks each material's redirect.
    pub active_holiday: ENPCHoliday,
    /// The client's asset checks.
    pub integrity: &'a mut ClientAssetIntegrity,
}

impl<'a> TileAssets<'a> {
    /// No active holiday, so no holiday redirects.
    pub fn new(loader: &'a AssetLoader, integrity: &'a mut ClientAssetIntegrity) -> Self {
        Self { loader, use_holiday_redirects: false, active_holiday: ENPCHoliday::NONE, integrity }
    }
}

/// `Landscape.TILE_SIZE`: a tile covers 1024 × 1024 world units.
pub const TILE_SIZE: f32 = 1024.0;
/// `Landscape.TILE_HEIGHT`: heights span 2048 units, from the tile origin at y = -1024.
pub const TILE_HEIGHT: f32 = 2048.0;
pub const HEIGHTMAP_RESOLUTION: usize = 257;
pub const SPLATMAP_RESOLUTION: usize = 256;
/// `SPLATMAP_COUNT * SPLATMAP_CHANNELS` = 2 × 4.
pub const SPLATMAP_LAYERS: usize = 8;
pub const HOLES_RESOLUTION: usize = 256;

// The most bytes each tile reader consumes: `readHeightmap` 2 per sample, `readSplatmap` 1 per layer, `ReadHoles` a
// version byte and 1 bit per cell. Only that much of a file is read.
const HEIGHTMAP_FILE_BYTES: usize = HEIGHTMAP_RESOLUTION * HEIGHTMAP_RESOLUTION * 2;
const SPLATMAP_FILE_BYTES: usize = SPLATMAP_RESOLUTION * SPLATMAP_RESOLUTION * SPLATMAP_LAYERS;
const HOLES_FILE_BYTES: usize = 1 + HOLES_RESOLUTION * HOLES_RESOLUTION / 8;

/// Assembly-qualified name of `SDG.Framework.Landscapes.Landscape`.
pub const LANDSCAPE_TYPE: &str =
    "SDG.Framework.Landscapes.Landscape, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null";

/// `LandscapeTile.DEFAULT_MATERIAL` ("Fallback_Green", 498ca625072d443a876b2a4f11896018), the first layer's default.
pub fn default_material() -> Guid {
    Guid::from_bytes([
        0x25, 0xa6, 0x8c, 0x49, 0x2d, 0x07, 0x3a, 0x44, 0x87, 0x6b, 0x2a, 0x4f, 0x11, 0x89, 0x60, 0x18,
    ])
}

/// `LandscapeCoord`: tile `(x, y)` covers world x from `x * 1024` and world z from `y * 1024`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LandscapeCoord {
    pub x: i32,
    pub y: i32,
}

impl fmt::Display for LandscapeCoord {
    /// `'(' + x.ToString() + ", " + y.ToString() + ')'`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// SHA-1 of the bytes each tile file contributed, as passed to `Level.includeHash`. `None` if the file was missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TileHashes {
    pub heightmap: Option<[u8; 20]>,
    pub splatmap: Option<[u8; 20]>,
    pub holes: Option<[u8; 20]>,
}

/// One landscape tile, with the original's arrays and their index order.
#[derive(Debug, Clone)]
pub struct LandscapeTile {
    pub coord: LandscapeCoord,
    /// Eight material references (`AssetReference<LandscapeMaterialAsset>` GUIDs).
    pub materials: Vec<Guid>,
    /// `heightmap[row][column]`: row follows world z, column follows world x. 0..1 of `TILE_HEIGHT`.
    pub heightmap: Box<[[f32; HEIGHTMAP_RESOLUTION]; HEIGHTMAP_RESOLUTION]>,
    /// `splatmap[row][column][layer]`, weights 0..1.
    pub splatmap: Box<[[[f32; SPLATMAP_LAYERS]; SPLATMAP_RESOLUTION]; SPLATMAP_RESOLUTION]>,
    /// `holes[row][column]`: true means solid ground (Unity's convention), false means a hole.
    pub holes: Box<[[bool; HOLES_RESOLUTION]; HOLES_RESOLUTION]>,
    pub has_any_holes_data: bool,
    pub hashes: TileHashes,
}

fn boxed_array<T: Clone, const N: usize>(value: T) -> Box<[T; N]> {
    let boxed: Box<[T]> = vec![value; N].into_boxed_slice();
    match boxed.try_into() {
        Ok(array) => array,
        Err(_) => unreachable!("vec has exactly N elements"),
    }
}

impl LandscapeTile {
    /// The `LandscapeTile` constructor's defaults: heights 0.5, layer 0 fully painted, no holes, Fallback_Green first.
    pub fn new(coord: LandscapeCoord) -> Self {
        let mut first_layer = [0.0f32; SPLATMAP_LAYERS];
        first_layer[0] = 1.0;
        let mut materials = vec![Guid::EMPTY; SPLATMAP_LAYERS];
        materials[0] = default_material();
        Self {
            coord,
            materials,
            heightmap: boxed_array([0.5f32; HEIGHTMAP_RESOLUTION]),
            splatmap: boxed_array([first_layer; SPLATMAP_RESOLUTION]),
            holes: boxed_array([true; HOLES_RESOLUTION]),
            has_any_holes_data: false,
            hashes: TileHashes::default(),
        }
    }
}

/// A file read through `SHA1Stream`: `ReadByte` returns -1 past the end, and the hash covers what was read.
struct HashedBytes<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> HashedBytes<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read_byte(&mut self) -> i32 {
        match self.bytes.get(self.pos) {
            Some(&b) => {
                self.pos += 1;
                i32::from(b)
            }
            None => -1,
        }
    }

    fn hash(&self) -> [u8; 20] {
        Sha1::digest(&self.bytes[..self.pos]).into()
    }
}

/// Paths to one map's files, plus the path string the original builds for its messages:
/// `Path.Combine(RootDirectory.FullName, "Maps")` + `\` + folder (from `Directory.GetDirectories`).
/// `FullName` is `Path.GetFullPath`: absolute, with `.` and `..` resolved.
struct MapPaths {
    folder: String,
    message_root: String,
}

impl MapPaths {
    fn new(install: &InstallRoot, folder: &str) -> Self {
        Self { folder: folder.to_string(), message_root: level_path_string(install, folder) }
    }

    fn rel(&self, sub: &str) -> String {
        format!("Maps/{}/{}", self.folder, sub)
    }

    fn message_path(&self, suffix: &str) -> String {
        format!("{}{}", self.message_root, suffix)
    }
}

/// `Path.Combine(PathEx.Join(RootDirectory, "Maps"))` and the folder that `Directory.GetDirectories` appends.
fn level_path_string(install: &InstallRoot, folder: &str) -> String {
    let separator = PathStyle::HOST.separator();
    // "Maps" never has an invalid character, and the root was checked when the install was opened.
    let maps = mono_path::combine(PathStyle::HOST, install.full_name(), "Maps")
        .unwrap_or_else(|_| format!("{}{separator}Maps", install.full_name()));
    format!("{maps}{separator}{folder}")
}

fn io_exception(error: io::Error) -> MonoException {
    MonoException::new(ExceptionKind::Io, error.to_string())
}

/// Reads a map's landscape. `None` when the map has no `Level.hierarchy` or no `Landscape` item (for example
/// legacy-ground maps). Messages go to `log` with the original's exact text.
pub fn read_landscape(
    install: &InstallRoot,
    map_folder: &str,
    assets: &mut TileAssets<'_>,
    log: &mut dyn LogSink,
) -> Result<Option<Vec<LandscapeTile>>, MonoException> {
    let map = MapPaths::new(install, map_folder);
    let hierarchy_rel = map.rel("Level.hierarchy");
    if !install.exists(&hierarchy_rel) {
        return Ok(None);
    }
    let bytes = install.read(&hierarchy_rel).map_err(io_exception)?;
    let table = read_kvt_bytes(&bytes)?;

    let mut landscape: Option<Vec<LandscapeTile>> = None;
    walk_hierarchy(&table, log, |item, mut item_reader, log| {
        if item.type_name.as_ref().is_some_and(|name| *name == LANDSCAPE_TYPE) {
            let instance_id = item.instance_id.unwrap_or(0);
            read_landscape_item(install, &map, instance_id, &mut item_reader, &mut landscape, assets, log)?;
        }
        Ok(())
    })?;
    Ok(landscape)
}

/// `Landscape.read`. The first `Landscape` item becomes the level's landscape; later ones are ignored with a warning.
fn read_landscape_item(
    install: &InstallRoot,
    map: &MapPaths,
    instance_id: u32,
    item_reader: &mut KvtCursor<'_>,
    landscape: &mut Option<Vec<LandscapeTile>>,
    assets: &mut TileAssets<'_>,
    log: &mut dyn LogSink,
) -> Result<(), MonoException> {
    let mut reader = item_reader.read_object()?.ok_or_else(MonoException::null_reference)?;
    let tile_count = reader.read_array_length("Tiles")?;
    if landscape.is_some() {
        log.log(
            LogLevel::Warn,
            &format!("Level contains multiple Landscapes. Ignoring {tile_count} tile(s) with instance ID: {instance_id}"),
        );
        return Ok(());
    }

    log.log(LogLevel::Info, &format!("Loading {tile_count} landscape tiles"));
    let mut tiles: Vec<LandscapeTile> = Vec::new();
    for tile_index in 0..tile_count {
        reader.read_array_index(tile_index);
        let tile = read_tile(install, map, &mut reader, assets, log)?;
        if tiles.iter().any(|t| t.coord == tile.coord) {
            log.log(LogLevel::Error, &format!("Duplicate landscape coord read: {}", tile.coord));
        } else {
            tiles.push(tile);
        }
    }
    *landscape = Some(tiles);
    Ok(())
}

/// `LandscapeTile.read`.
fn read_tile(
    install: &InstallRoot,
    map: &MapPaths,
    reader: &mut KvtCursor<'_>,
    assets: &mut TileAssets<'_>,
    log: &mut dyn LogSink,
) -> Result<LandscapeTile, MonoException> {
    let mut tile_reader = reader.read_object()?.ok_or_else(MonoException::null_reference)?;

    // readValue<LandscapeCoord>("Coord") → LandscapeCoord.read
    tile_reader.read_key("Coord");
    let mut coord_reader = tile_reader.read_object()?.ok_or_else(MonoException::null_reference)?;
    coord_reader.read_key("X");
    let x = coord_reader.read_i32()?;
    coord_reader.read_key("Y");
    let y = coord_reader.read_i32()?;
    let mut tile = LandscapeTile::new(LandscapeCoord { x, y });

    let layer_count = tile_reader.read_array_length("Materials")?;
    for layer in 0..layer_count {
        // readValue<AssetReference<LandscapeMaterialAsset>>(layer) → AssetReference.read
        tile_reader.read_array_index(layer);
        let guid = match tile_reader.read_object()? {
            None => tile_reader.read_guid()?,
            Some(mut nested) => {
                nested.read_key("GUID");
                nested.read_guid()?
            }
        };
        let reference = check_material(AssetReference { guid }, tile.coord, layer, assets, log);
        // `materials[layer] = materialRef`, after the checks.
        let slot = usize::try_from(layer)
            .ok()
            .and_then(|l| tile.materials.get_mut(l))
            .ok_or_else(MonoException::list_index_out_of_range)?;
        *slot = reference.guid;
    }
    update_prototypes(&tile, assets, log);

    read_heightmap(install, map, &mut tile, log)?;
    read_splatmap(install, map, &mut tile, log)?;
    read_holes(install, map, &mut tile)?;
    Ok(tile)
}

/// `LandscapeTile.read`'s checks of a material reference from the map, when it is valid: with holiday redirects on,
/// the material's redirect for the active holiday replaces it when that redirect is valid; then a reference the
/// assets can't find is recorded as known missing. The editor's asset redirector applies only in the editor.
fn check_material(
    mut reference: AssetReference,
    coord: LandscapeCoord,
    layer: i32,
    assets: &mut TileAssets<'_>,
    log: &mut dyn LogSink,
) -> AssetReference {
    if !reference.is_valid() {
        return reference;
    }
    if assets.use_holiday_redirects
        && let Some((_, original)) = assets.loader.find_landscape_material(reference, log)
    {
        let redirected = original.holiday_redirect(assets.active_holiday);
        if redirected.is_valid() {
            reference = redirected;
        }
    }
    if assets.loader.find_landscape_material(reference, log).is_none() {
        assets.integrity.server_add_known_missing_asset(reference.guid, &tile_context(coord, layer), log);
    }
    reference
}

/// `updatePrototypes` on a client, without the Unity terrain layers (M2e): every layer's material is looked up, and
/// each valid reference is queued for the server's asset check.
fn update_prototypes(tile: &LandscapeTile, assets: &mut TileAssets<'_>, log: &mut dyn LogSink) {
    for (layer, &guid) in tile.materials.iter().enumerate() {
        let reference = AssetReference { guid };
        let material = assets.loader.find_landscape_material(reference, log).map(|(index, _)| index);
        if reference.is_valid() {
            assets.integrity.queue_request(guid, material, &tile_context(tile.coord, layer), log);
        }
    }
}

/// The context both checks give: the tile's coordinates and the layer, as integers in the invariant culture (V13).
fn tile_context(coord: LandscapeCoord, layer: impl fmt::Display) -> String {
    format!("Landscape tile (x: {} y: {} layer: {layer})", coord.x, coord.y)
}

fn tile_file_name(coord: LandscapeCoord, suffix: &str, extension: &str) -> String {
    format!("Tile_{}_{}{suffix}{extension}", coord.x, coord.y)
}

/// `LandscapeTile.readHeightmap("_Source", heightmap)`: big-endian u16 samples, row-major.
fn read_heightmap(
    install: &InstallRoot,
    map: &MapPaths,
    tile: &mut LandscapeTile,
    log: &mut dyn LogSink,
) -> Result<(), MonoException> {
    let file_name = tile_file_name(tile.coord, "_Source", ".heightmap");
    let rel = map.rel(&format!("Landscape/Heightmaps/{file_name}"));
    if !install.exists(&rel) {
        let path = map.message_path(&format!("/Landscape/Heightmaps/{file_name}"));
        log.log(LogLevel::Warn, &format!("LandscapeTile missing heightmap file: {path}"));
        return Ok(());
    }
    let bytes = install.read_prefix(&rel, HEIGHTMAP_FILE_BYTES).map_err(io_exception)?;
    let mut stream = HashedBytes::new(&bytes);
    for row in tile.heightmap.iter_mut() {
        for sample in row.iter_mut() {
            let high = stream.read_byte();
            let low = stream.read_byte();
            let raw = ((high << 8) | low) as u16;
            *sample = f32::from(raw) / f32::from(u16::MAX);
        }
    }
    tile.hashes.heightmap = Some(stream.hash());
    Ok(())
}

/// `LandscapeTile.readSplatmap("_Source", splatmap)`: one byte per layer, layers innermost.
fn read_splatmap(
    install: &InstallRoot,
    map: &MapPaths,
    tile: &mut LandscapeTile,
    log: &mut dyn LogSink,
) -> Result<(), MonoException> {
    let file_name = tile_file_name(tile.coord, "_Source", ".splatmap");
    let rel = map.rel(&format!("Landscape/Splatmaps/{file_name}"));
    if !install.exists(&rel) {
        let path = map.message_path(&format!("/Landscape/Splatmaps/{file_name}"));
        log.log(LogLevel::Warn, &format!("LandscapeTile missing splatmap file: {path}"));
        return Ok(());
    }
    let bytes = install.read_prefix(&rel, SPLATMAP_FILE_BYTES).map_err(io_exception)?;
    let mut stream = HashedBytes::new(&bytes);
    for row in tile.splatmap.iter_mut() {
        for texel in row.iter_mut() {
            for weight in texel.iter_mut() {
                let raw = stream.read_byte() as u8;
                *weight = f32::from(raw) / f32::from(u8::MAX);
            }
        }
    }
    tile.hashes.splatmap = Some(stream.hash());
    Ok(())
}

/// `LandscapeTile.ReadHoles`: a version byte, then 8 cells per byte, least significant bit first. A missing file is
/// normal (old maps, maps without holes) and logs nothing.
fn read_holes(install: &InstallRoot, map: &MapPaths, tile: &mut LandscapeTile) -> Result<(), MonoException> {
    let file_name = tile_file_name(tile.coord, "", ".bin");
    let rel = map.rel(&format!("Landscape/Holes/{file_name}"));
    if !install.exists(&rel) {
        return Ok(());
    }
    let bytes = install.read_prefix(&rel, HOLES_FILE_BYTES).map_err(io_exception)?;
    let mut stream = HashedBytes::new(&bytes);
    let _version = stream.read_byte();
    for row in tile.holes.iter_mut() {
        for group in row.chunks_mut(8) {
            let value = stream.read_byte() as u8;
            for (offset, cell) in group.iter_mut().enumerate() {
                *cell = (value & (1 << offset)) > 0;
            }
        }
    }
    tile.hashes.holes = Some(stream.hash());
    tile.has_any_holes_data = true;
    Ok(())
}
