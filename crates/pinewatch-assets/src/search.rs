//! The client's asset search locations, in the order `Assets.LoadAllAssets` requests them (`Bundles/Assets.cs`,
//! `Level/Level.cs`).
//!
//! Pinewatch runs as the shipped Windows client (`WITH_NOREDIST`, V21) with the workshop service present but
//! subscribed to nothing, so there are no workshop locations and `ScanKnownLevels` never reaches its legacy server
//! branch.

use pinewatch_io::{InstallRoot, LogLevel, LogSink, log_exception};
use pinewatch_mono::MonoException;

use crate::origin::{OriginId, Origins};
use crate::paths::{self, MonoPath};

/// A folder the asset worker searches, with the origin its assets get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchLocation {
    pub dir: MonoPath,
    pub origin: OriginId,
}

/// The part of `LevelInfo` the asset search needs: the folder and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelInfo {
    pub name: String,
    pub dir: MonoPath,
}

/// The search locations `LoadAllAssets` adds on the client, in order, with their log messages. An exception the C#
/// doesn't catch there (from enumerating `Sandbox`) is returned.
pub fn client_search_locations(
    install: &InstallRoot,
    origins: &mut Origins,
    log: &mut dyn LogSink,
) -> Result<Vec<SearchLocation>, MonoException> {
    let root = MonoPath::root(install);
    let mut locations = Vec::new();

    // `Path.Combine(ReadWrite.PATH, "Bundles")`, added without checking that it exists (V21).
    let core = origins.core();
    add_search_location(&mut locations, root.combine("Bundles")?, core, origins, log)?;

    // `AddClientUgcSearchLocations`: no subscribed workshop content.

    add_sandbox_search_locations(install, &root, &mut locations, origins, log)?;
    add_map_search_locations(install, &root, &mut locations, origins, log)?;
    Ok(locations)
}

/// `AddSearchLocation`: the full path, logged, then requested.
fn add_search_location(
    locations: &mut Vec<SearchLocation>,
    dir: MonoPath,
    origin: OriginId,
    origins: &Origins,
    log: &mut dyn LogSink,
) -> Result<(), MonoException> {
    let dir = dir.full_path()?;
    log.log(LogLevel::Info, &format!("{} added asset search location \"{}\"", origins.get(origin).name, dir.full));
    locations.push(SearchLocation { dir, origin });
    Ok(())
}

/// `AddSandboxSearchLocations`. When `Sandbox` doesn't exist the original creates it; Pinewatch doesn't (L3).
fn add_sandbox_search_locations(
    install: &InstallRoot,
    root: &MonoPath,
    locations: &mut Vec<SearchLocation>,
    origins: &mut Origins,
    log: &mut dyn LogSink,
) -> Result<(), MonoException> {
    let sandbox = root.combine("Sandbox")?;
    if !install.is_dir(&sandbox.rel) {
        return Ok(());
    }
    for folder in paths::directories(install, &sandbox)? {
        let folder_name = pinewatch_io::mono_path::get_file_name(paths::STYLE, &folder.full)?;
        log.log(LogLevel::Info, &format!("Sandbox: {folder_name}"));
        let origin = origins.add_sandbox(&folder_name);
        add_search_location(locations, folder, origin, origins, log)?;
    }
    Ok(())
}

/// `AddMapSearchLocations`: each known level's `Bundles` folder, if it exists.
fn add_map_search_locations(
    install: &InstallRoot,
    root: &MonoPath,
    locations: &mut Vec<SearchLocation>,
    origins: &mut Origins,
    log: &mut dyn LogSink,
) -> Result<(), MonoException> {
    for level in scan_known_levels(install, root, log)? {
        let bundles = level.dir.combine("Bundles")?;
        if install.is_dir(&bundles.rel) {
            let origin = origins.find_or_add_level(&level.name);
            add_search_location(locations, bundles, origin, origins, log)?;
        }
    }
    Ok(())
}

/// `ScanKnownLevels` on its first call, for the root `Maps` folder: each folder `ReadLevelInfo` accepts, logged as
/// discovered. An exception while listing ends the scan and is logged.
pub fn scan_known_levels(
    install: &InstallRoot,
    root: &MonoPath,
    log: &mut dyn LogSink,
) -> Result<Vec<LevelInfo>, MonoException> {
    let maps = root.combine("Maps")?;
    let mut levels = Vec::new();
    match paths::directories(install, &maps) {
        Ok(folders) => {
            for folder in folders {
                if let Some(level) = read_level_info(install, folder, log) {
                    log.log(LogLevel::Info, &format!("Discovered level \"{}\" at \"{}\"", level.name, level.dir.full));
                    levels.push(level);
                }
            }
        }
        Err(exception) => log_exception(log, &exception, "Caught exception loading levels in root Maps folder:"),
    }
    Ok(levels)
}

/// `ReadLevelInfo` as far as the asset search needs it: `None` without `Level.dat` or when reading it throws.
/// `Level.dat`'s fields and `Config.json` come with level info (V23).
fn read_level_info(install: &InstallRoot, dir: MonoPath, log: &mut dyn LogSink) -> Option<LevelInfo> {
    let read = || -> Result<Option<LevelInfo>, MonoException> {
        let level_file = dir.combine("Level.dat")?;
        if !install.exists(&level_file.rel) {
            return Ok(None);
        }
        // `ReadWrite.readBlock` reads the whole file; reading a `Block` never throws.
        install.read(&level_file.rel).map_err(|error| MonoException::new(pinewatch_mono::ExceptionKind::Io, error.to_string()))?;
        let name = paths::folder_name(&dir.full)?;
        Ok(Some(LevelInfo { name, dir: dir.clone() }))
    };
    match read() {
        Ok(level) => level,
        Err(exception) => {
            log_exception(log, &exception, &format!("Caught exception reading level info file ({}):", dir.full));
            None
        }
    }
}
