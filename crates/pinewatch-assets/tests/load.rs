//! The main thread's side: master bundles as they arrive, and `LoadFile` up to the asset's header (M2c task 6).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{AssetLoader, AssetType, MonoPath, Origins, SearchLocation, read_location};
use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};
use pinewatch_mono::Guid;
use sha1::{Digest, Sha1};

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SEPARATOR: &str = if cfg!(windows) { "\\" } else { "/" };
const GUID: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";
const CORE_CONFIG: &str = "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 6";

fn temp_install(files: &[(&str, &str)]) -> (TempInstall, InstallRoot) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-load-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Bundles")).unwrap();
    for (file, contents) in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(&root).unwrap();
    (TempInstall(root), install)
}

/// An install with the core master bundle and `files`, loaded through the worker and the loader.
fn load(files: &[(&str, &str)]) -> (TempInstall, InstallRoot, AssetLoader, MemoryLog) {
    let mut all = vec![("Bundles/MasterBundle.dat", CORE_CONFIG), ("Bundles/core.masterbundle", "")];
    all.extend_from_slice(files);
    load_without_core(&all)
}

fn load_without_core(files: &[(&str, &str)]) -> (TempInstall, InstallRoot, AssetLoader, MemoryLog) {
    let (dir, install) = temp_install(files);
    let origins = Origins::new();
    let bundles = MonoPath { full: format!("{}{SEPARATOR}Bundles", install.full_name()), rel: "Bundles".to_string() };
    let location = SearchLocation { dir: bundles, origin: origins.core() };
    let results = read_location(&install, &location, "English").unwrap();
    let mut log = MemoryLog::default();
    let mut loader = AssetLoader::new(origins);
    loader.take_results(results, &mut log);
    (dir, install, loader, log)
}

fn full(install: &InstallRoot, rel: &str) -> String {
    format!("{}{SEPARATOR}{}", install.full_name(), rel.replace('/', SEPARATOR))
}

fn warnings(log: &MemoryLog) -> Vec<String> {
    log.entries.iter().filter(|(level, _)| *level == LogLevel::Warn).map(|(_, text)| text.clone()).collect()
}

fn sha1(bytes: &[u8]) -> [u8; 20] {
    Sha1::digest(bytes).into()
}

#[test]
fn legacy_definition_in_the_core_master_bundle() {
    let contents = format!("GUID {GUID}\nType Gun\nID 7");
    let (_dir, install, loader, log) = load(&[("Bundles/Items/Gun/Gun.dat", &contents)]);
    assert!(loader.core_master_bundle().is_some());
    assert_eq!(loader.headers.len(), 1, "{:?}", log.entries);
    let header = &loader.headers[0];
    assert_eq!(header.internal_name, "Gun");
    assert_eq!(header.name, "Gun");
    assert_eq!(header.guid, Guid::try_parse(Some(&pinewatch_mono::NetString::from(GUID))).unwrap());
    assert_eq!(header.asset_type, AssetType::by_name("ItemGunAsset").unwrap());
    assert_eq!(header.id, 7);
    assert_eq!(header.master_bundle, loader.core_master_bundle());
    assert_eq!(header.bundle_relative_path.as_deref(), Some("/Items/Gun"));
    // `Hash.combine(fileHash, Hash.SHA1("/items/gun/gun"))`.
    let mut both = sha1(contents.as_bytes()).to_vec();
    both.extend_from_slice(&sha1(b"/items/gun/gun"));
    assert_eq!(header.hash, sha1(&both));
    assert!(!header.has_errors && !header.convert_shaders_to_standard && !header.consolidate_shaders);
    assert!(warnings(&log).is_empty(), "{:?}", log.entries);
    let _ = install;
}

#[test]
fn asset_dat_takes_the_folder_name_and_v2_uses_the_asset_dictionary() {
    let v2 = format!(
        "Metadata\n{{\n\tGUID {GUID}\n\tType \"SDG.Unturned.LandscapeMaterialAsset, Assembly-CSharp\"\n}}\nAsset\n{{\n\tID 5\n}}"
    );
    let (_dir, _install, loader, log) = load(&[
        ("Bundles/Items/Hat/Asset.dat", "GUID 2b2a3d4c5e6f708192a3b4c5d6e7f809\nType Hat"),
        ("Bundles/Materials/Grass.asset", &v2),
    ]);
    assert_eq!(loader.headers.len(), 2, "{:?}", log.entries);
    let hat = loader.headers.iter().find(|header| header.internal_name == "Hat").unwrap();
    assert_eq!(hat.asset_type, AssetType::by_name("ItemHatAsset").unwrap());
    let grass = loader.headers.iter().find(|header| header.internal_name == "Grass").unwrap();
    assert_eq!(grass.asset_type, AssetType::by_name("LandscapeMaterialAsset").unwrap());
    assert_eq!(grass.id, 5);
    assert!(grass.uses_asset_dictionary);
}

#[test]
fn header_errors_in_order() {
    let cases: Vec<(String, &str)> = vec![
        ("GUID abc\nType Gun".to_string(), "Unable to parse GUID in \"{path}\""),
        ("Metadata\n{\n\tGUID abc\n}".to_string(), "Unable to parse Metadata.GUID in \"{path}\""),
        (format!("Metadata\n{{\n\tGUID {GUID}\n}}"), "Unable to parse Metadata.Type in \"{path}\""),
        (format!("GUID {GUID}"), "Missing asset Type in \"{path}\""),
        (format!("GUID {GUID}\nType Banana"), "Unhandled asset type \"Banana\" in \"{path}\""),
        ("GUID 00000000000000000000000000000000\nType Gun".to_string(), "Cannot use empty GUID in \"{path}\""),
        (format!("GUID {GUID}\nType gun"), "Unhandled asset type \"gun\" in \"{path}\""),
    ];
    for (contents, expected) in cases {
        let (_dir, install, loader, log) = load(&[("Bundles/X/X.dat", &contents)]);
        let path = full(&install, "Bundles/X/X.dat");
        assert_eq!(warnings(&log), [expected.replace("{path}", &path)], "{contents}");
        assert_eq!(loader.errors, [expected.replace("{path}", &path)]);
        assert!(loader.headers.is_empty());
    }
}

#[test]
fn parser_errors_come_first_and_mark_the_asset() {
    let contents = format!("GUID {GUID}\nType Gun\nID 1\n}}");
    let (_dir, install, loader, log) = load(&[("Bundles/X/X.dat", &contents)]);
    let path = full(&install, "Bundles/X/X.dat");
    assert_eq!(warnings(&log), [format!("Error parsing \"{path}\": \"unexpected end of dictionary/object '}}' on line 4\"")]);
    assert!(loader.headers[0].has_errors);
}

#[test]
fn master_bundle_override_and_exclusion() {
    let (_dir, install, loader, log) = load(&[
        ("Bundles/A/A.dat", &format!("GUID {GUID}\nType Gun\nID 1\nMaster_Bundle_Override missing.masterbundle")),
        ("Bundles/B/B.dat", "GUID 3b2a3d4c5e6f708192a3b4c5d6e7f809\nType Gun\nID 2\nExclude_From_Master_Bundle"),
        ("Bundles/C/C.dat", "GUID 4b2a3d4c5e6f708192a3b4c5d6e7f809\nType Gun\nID 3\nMaster_Bundle_Override CORE.MASTERBUNDLE"),
    ]);
    let path = full(&install, "Bundles/A/A.dat");
    assert_eq!(warnings(&log), [format!("Unable to find master bundle override 'missing.masterbundle' for '{path}'")]);
    let by_name = |name: &str| loader.headers.iter().find(|header| header.internal_name == name).unwrap();
    assert_eq!(by_name("A").master_bundle, None);
    assert_eq!(by_name("B").master_bundle, None);
    assert_eq!(by_name("C").master_bundle, loader.core_master_bundle());
    // Outside a master bundle the hash is the file's alone.
    let contents = format!("GUID {GUID}\nType Gun\nID 1\nMaster_Bundle_Override missing.masterbundle");
    assert_eq!(by_name("A").hash, sha1(contents.as_bytes()));
}

#[test]
fn individual_asset_bundle_versions() {
    let (_dir, _install, loader, log) = load(&[
        ("Bundles/Low/Low.dat", &format!("GUID {GUID}\nType Gun\nID 1\nAsset_Bundle_Version 0\nExclude_From_Master_Bundle")),
        ("Bundles/High/High.dat", "GUID 5b2a3d4c5e6f708192a3b4c5d6e7f809\nType Gun\nID 2\nAsset_Bundle_Version 9"),
    ]);
    let mut found = warnings(&log);
    found.sort();
    assert_eq!(found, [
        "High Highest individual asset bundle version is 6, associated with 2022 LTS.",
        "Low Lowest individual asset bundle version is 1 (default), associated with 5.5.",
    ]);
    let low = loader.headers.iter().find(|header| header.internal_name == "Low").unwrap();
    assert!(low.convert_shaders_to_standard && low.consolidate_shaders);
}

/// The original writes a new GUID into a definition that has none; Pinewatch uses a new one in memory and writes
/// nothing (L3).
#[test]
fn missing_guid_is_assigned_in_memory_only() {
    let (dir, _install, loader, log) = load(&[("Bundles/X/X.dat", "Type Gun")]);
    assert_eq!(loader.headers.len(), 1, "{:?}", log.entries);
    assert_ne!(loader.headers[0].guid, Guid::EMPTY);
    assert_eq!(fs::read_to_string(dir.0.join("Bundles/X/X.dat")).unwrap(), "Type Gun");
}

/// `Activator.CreateInstance` on an abstract class throws `MissingMethodException` with Mono's odd message.
/// The shape of the shipped `Bundles/Assets/Weather/DefaultRain.asset`. `WeatherAssetBase.PopulateAsset` and
/// `WeatherAsset.PopulateAsset` can't throw for it (every value has a default), so the game indexes it; their own
/// fields aren't ported yet (V28).
#[test]
fn weather_asset_is_created_and_indexed() {
    let contents = format!(
        "// Default has no overrides.\nMetadata\n{{\n\tGUID {GUID}\n\tType SDG.Unturned.WeatherAsset, Assembly-CSharp, \
         Version=0.0.0.0, Culture=neutral, PublicKeyToken=null\n}}\nAsset\n{{\n\tComponent_Type \
         SDG.Unturned.LegacyRainComponent, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null\n\t\
         Volume_Mask 1\n\tFade_In_Duration 20\n}}"
    );
    let (_dir, _install, loader, log) = load(&[("Bundles/Assets/Weather/DefaultRain.asset", &contents)]);
    assert_eq!(warnings(&log), Vec::<String>::new());
    assert!(loader.errors.is_empty());
    assert_eq!(loader.headers.len(), 1);
    let header = &loader.headers[0];
    assert_eq!(header.asset_type, AssetType::by_name("WeatherAsset").unwrap());
    assert_eq!(header.friendly_name, "DefaultRain");
    let guid = Guid::try_parse(Some(&pinewatch_mono::NetString::from(GUID))).unwrap();
    assert_eq!(loader.find(guid, &mut MemoryLog::default()), Some(0));
}

#[test]
fn abstract_type_cannot_be_constructed() {
    let contents =
        format!("Metadata\n{{\n\tGUID {GUID}\n\tType \"SDG.Framework.Foliage.FoliageInfoAsset, Assembly-CSharp\"\n}}");
    let (_dir, install, loader, log) = load(&[("Bundles/F/F.asset", &contents)]);
    let path = full(&install, "Bundles/F/F.asset");
    let message = "Method 'Cannot create an abstract class '{0}'..SDG.Framework.Foliage.FoliageInfoAsset' not found.";
    assert_eq!(warnings(&log), [format!(
        "Caught exception while constructing SDG.Framework.Foliage.FoliageInfoAsset in \"{path}\": {message}"
    )]);
    assert_eq!(log.entries.last().unwrap(), &(LogLevel::Error, message.to_string()));
    assert!(loader.headers.is_empty());
}

/// `TryGetString` finds a flag with a null value, so `Bundle_Override_Path` as a flag makes `ToLowerInvariant`
/// throw, which `TryLoadFile` logs.
#[test]
fn bundle_override_path_flag_throws() {
    let (_dir, install, loader, log) = load(&[("Bundles/X/X.dat", &format!("GUID {GUID}\nType Gun\nBundle_Override_Path"))]);
    let path = full(&install, "Bundles/X/X.dat");
    assert_eq!(log.entries[log.entries.len() - 2..], [
        (LogLevel::Error, format!("Exception loading file {path}:")),
        (LogLevel::Error, "Object reference not set to an instance of an object".to_string()),
    ]);
    assert!(loader.headers.is_empty());
}

#[test]
fn legacy_bundle_override_path_outside_master_bundles() {
    let (_dir, _install, loader, _log) = load(&[(
        "Bundles/X/X.dat",
        &format!("GUID {GUID}\nType Gun\nExclude_From_Master_Bundle\nBundle_Override_Path Old/Path"),
    )]);
    let header = &loader.headers[0];
    assert_eq!(header.legacy_bundle_path.as_deref(), Some("Old/Path/Path.unity3d"));
    assert_eq!(header.name, "X");
}

#[test]
fn long_paths_are_reported() {
    // Three folders of 90 characters: over 260 in all, while each name stays within file-system limits.
    let segment = "N".repeat(90);
    let rel = format!("Bundles/{segment}/{segment}/{segment}/{segment}.dat");
    let (_dir, install, loader, log) = load(&[(&rel, &format!("GUID {GUID}\nType Gun"))]);
    let path = full(&install, &rel);
    assert_eq!(
        warnings(&log)[0],
        format!("Asset path exceeds 260 characters and might not load properly on Windows: \"{path}\"")
    );
    assert_eq!(loader.headers.len(), 1);
}

/// Without a core master bundle the main thread never loads a definition (the original waits forever).
#[test]
fn definitions_wait_for_the_core_master_bundle() {
    let (_dir, _install, loader, _log) = load_without_core(&[("Bundles/X/X.dat", &format!("GUID {GUID}\nType Gun"))]);
    assert!(loader.core_master_bundle().is_none());
    assert!(loader.headers.is_empty());
    assert_eq!(loader.pending(), 1);
}

/// The header's `Local` reads the definition's translation, with the English fallback when the language isn't
/// English.
#[test]
fn header_localization_reads_the_translation() {
    let (_dir, _install, loader, _log) = load(&[
        ("Bundles/Items/Gun/Gun.dat", &format!("GUID {GUID}\nType Gun")),
        ("Bundles/Items/Gun/English.dat", "Name Eaglefire\nDescription Rifle"),
    ]);
    let local = loader.headers[0].localization();
    assert_eq!(local.format("Name").to_string_lossy(), "Eaglefire");
    assert_eq!(local.format("Missing").to_string_lossy(), "Missing");
}
