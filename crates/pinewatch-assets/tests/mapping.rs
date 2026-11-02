//! The asset index: `AddToMapping`'s rules and messages, with each asset's friendly name and category (M2c task 9).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{AssetLoader, EAssetType, Origins, client_search_locations, read_location};
use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};
use pinewatch_mono::{Guid, NetString};

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SEPARATOR: &str = if cfg!(windows) { "\\" } else { "/" };
const G1: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";
const G2: &str = "2b2a3d4c5e6f708192a3b4c5d6e7f809";
const CORE: [(&str, &str); 2] = [
    ("Bundles/MasterBundle.dat", "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 6"),
    ("Bundles/core.masterbundle", ""),
];

/// Loads every client search location of a made-up install (core `Bundles` first, then `Sandbox` folders).
fn load(files: &[(&str, &str)]) -> (TempInstall, AssetLoader, MemoryLog) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-mapping-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Maps")).unwrap();
    for (file, contents) in CORE.iter().chain(files) {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(&root).unwrap();
    let mut origins = Origins::new();
    let mut log = MemoryLog::default();
    let locations = client_search_locations(&install, &mut origins, &mut log).unwrap();
    let mut loader = AssetLoader::new(origins);
    for location in &locations {
        loader.take_results(read_location(&install, location, "English").unwrap(), &mut log);
    }
    (TempInstall(root), loader, log)
}

fn guid(text: &str) -> Guid {
    Guid::try_parse(Some(&NetString::from(text))).unwrap()
}

fn warnings(log: &MemoryLog) -> Vec<String> {
    log.entries.iter().filter(|(level, _)| *level == LogLevel::Warn).map(|(_, text)| text.clone()).collect()
}

fn index_of(loader: &AssetLoader, internal_name: &str) -> usize {
    loader.headers.iter().position(|header| header.internal_name == internal_name).unwrap()
}

/// The legacy ID goes in first, so a GUID conflict leaves the second asset in the ID table only.
#[test]
fn guid_conflict_reports_the_second_asset() {
    let (_dir, loader, log) = load(&[
        ("Bundles/A/A.dat", &format!("GUID {G1}\nType Gun\nID 1")),
        ("Bundles/A/English.dat", "Name Alpha"),
        ("Bundles/A/B/B.dat", &format!("GUID {G1}\nType Gun\nID 2")),
        ("Bundles/A/B/English.dat", "Name Beta"),
    ]);
    assert_eq!(warnings(&log), [format!(
        "Vanilla Built-in Assets Beta (Gun Item) [{G1}]: GUID already taken by Alpha (Gun Item) in Vanilla Built-in Assets!"
    )]);
    let mapping = loader.mapping();
    let (a, b) = (index_of(&loader, "A"), index_of(&loader, "B"));
    assert_eq!(mapping.find_by_guid(&guid(G1)), Some(a));
    assert_eq!(mapping.find_legacy(EAssetType::Item, 2), Some(b));
    assert_eq!(mapping.asset_list(), [a]);
    assert!(loader.headers[b].has_errors && !loader.headers[a].has_errors);
}

#[test]
fn legacy_id_conflict_stops_before_the_guid() {
    let (_dir, loader, log) = load(&[
        ("Bundles/A/A.dat", &format!("GUID {G1}\nType Gun\nID 5")),
        ("Bundles/A/English.dat", "Name Alpha"),
        ("Bundles/A/B/B.dat", &format!("GUID {G2}\nType Gun\nID 5")),
        ("Bundles/A/B/English.dat", "Name Beta"),
    ]);
    assert_eq!(warnings(&log), [format!(
        "Vanilla Built-in Assets Beta (Gun Item) [{G2}]: legacy ID 5 already taken by Alpha (Gun Item) in Vanilla Built-in Assets!"
    )]);
    let mapping = loader.mapping();
    assert_eq!(mapping.find_legacy(EAssetType::Item, 5), Some(index_of(&loader, "A")));
    assert_eq!(mapping.find_by_guid(&guid(G2)), None);
}

/// Items need an ID unless they are `Pro`; effects, vehicles, spawns and NPC assets don't; other categories do;
/// assets without a category skip the legacy table. The message doesn't stop the GUID from being added.
#[test]
fn which_assets_need_a_non_zero_id() {
    let (_dir, loader, log) = load(&[
        ("Bundles/Gun/Gun.dat", &format!("GUID {G1}\nType Gun")),
        ("Bundles/ProGun/ProGun.dat", "GUID 3b2a3d4c5e6f708192a3b4c5d6e7f809\nType Gun\nPro"),
        ("Bundles/Effect/Effect.dat", "GUID 4b2a3d4c5e6f708192a3b4c5d6e7f809\nType Effect"),
        ("Bundles/Tree/Tree.dat", "GUID 5b2a3d4c5e6f708192a3b4c5d6e7f809\nType Resource"),
        ("Bundles/Grass_Material/Grass_Material.dat", "GUID 6b2a3d4c5e6f708192a3b4c5d6e7f809\nType Tag"),
    ]);
    let mut found = warnings(&log);
    found.sort();
    assert_eq!(found, [
        format!("Vanilla Built-in Assets Gun (Gun Item) [{G1}]: needs a non-zero ID"),
        "Vanilla Built-in Assets Name (Resource) [5b2a3d4c5e6f708192a3b4c5d6e7f809]: needs a non-zero ID".to_string(),
    ]);
    assert_eq!(loader.mapping().find_by_guid(&guid(G1)), Some(index_of(&loader, "Gun")));
    assert_eq!(loader.mapping().asset_list().len(), 5);
}

/// Objects skip the checks: ID 0 is a normal key, and a taken ID is skipped silently.
#[test]
fn objects_skip_the_legacy_checks() {
    let (_dir, loader, log) = load(&[
        ("Bundles/A/A.dat", &format!("GUID {G1}\nType Large")),
        ("Bundles/A/B/B.dat", &format!("GUID {G2}\nType Medium")),
    ]);
    assert!(warnings(&log).is_empty(), "{:?}", log.entries);
    let mapping = loader.mapping();
    assert_eq!(mapping.find_legacy(EAssetType::Object, 0), Some(index_of(&loader, "A")));
    assert_eq!(mapping.find_by_guid(&guid(G2)), Some(index_of(&loader, "B")));
}

/// A sandbox origin overrides IDs: its asset replaces the core asset with the same ID and GUID, silently.
#[test]
fn overriding_origin_replaces_entries() {
    let (_dir, loader, log) = load(&[
        ("Bundles/Gun/Gun.dat", &format!("GUID {G1}\nType Gun\nID 7")),
        ("Sandbox/Mod/Gun2/Gun2.dat", &format!("GUID {G1}\nType Gun\nID 7")),
    ]);
    assert!(warnings(&log).is_empty(), "{:?}", log.entries);
    let (core, sandbox) = (index_of(&loader, "Gun"), index_of(&loader, "Gun2"));
    let mapping = loader.mapping();
    assert_eq!(mapping.find_by_guid(&guid(G1)), Some(sandbox));
    assert_eq!(mapping.find_legacy(EAssetType::Item, 7), Some(sandbox));
    assert_eq!(mapping.asset_list(), [sandbox]);
    assert!(loader.headers[core].has_been_replaced && !loader.headers[sandbox].has_been_replaced);
    assert_eq!(mapping.modification_counter(), 2);
}

#[test]
fn redirector_category_and_messages() {
    let (_dir, loader, log) = load(&[
        ("Bundles/R/R.dat", &format!("GUID {G1}\nType Redirector\nID 9\nAssetCategory ITEM\nTargetAsset {G2}")),
        ("Bundles/R/S/S.dat", "GUID 7b2a3d4c5e6f708192a3b4c5d6e7f809\nType Redirector\nID 3"),
    ]);
    let s = "7b2a3d4c5e6f708192a3b4c5d6e7f809";
    assert_eq!(warnings(&log), [
        format!("Vanilla Built-in Assets S (Redirector) [{s}]: legacy ID was assigned but AssetCategory was not"),
        format!("Vanilla Built-in Assets S (Redirector) [{s}]: unable to parse TargetAsset"),
    ]);
    let mapping = loader.mapping();
    assert_eq!(mapping.find_legacy(EAssetType::Item, 9), Some(index_of(&loader, "R")));
    assert_eq!(loader.headers[index_of(&loader, "R")].target_guid, Some(guid(G2)));
}

/// `AssetCategory 42` parses as an `EAssetType` no member has; the legacy table lookup then throws
/// `KeyNotFoundException`, which `LoadFile` reports as a failure to populate.
#[test]
fn undefined_category_fails_while_populating() {
    let (dir, loader, log) =
        load(&[("Bundles/R/R.dat", &format!("GUID {G1}\nType Redirector\nID 1\nAssetCategory 42\nTargetAsset {G2}"))]);
    let path = format!("{}{SEPARATOR}Bundles{SEPARATOR}R{SEPARATOR}R.dat", InstallRoot::open(&dir.0).unwrap().full_name());
    let message = "The given key '42' was not present in the dictionary.";
    assert_eq!(warnings(&log), [format!("Caught exception while populating \"{path}\": {message}")]);
    assert_eq!(log.entries.last().unwrap(), &(LogLevel::Error, message.to_string()));
    assert_eq!(loader.mapping().find_by_guid(&guid(G1)), None);
}

#[test]
fn friendly_names_follow_each_class() {
    let (_dir, loader, _log) = load(&[
        ("Bundles/Grass_Material/Grass_Material.asset", &format!(
            "Metadata\n{{\n\tGUID {G1}\n\tType \"SDG.Unturned.LandscapeMaterialAsset, Assembly-CSharp\"\n}}"
        )),
        ("Bundles/Road/Road.dat", "GUID 3b2a3d4c5e6f708192a3b4c5d6e7f809\nType Road"),
        ("Bundles/Road/English.dat", "Name Asphalt"),
        ("Bundles/Tag/Tag.dat", "GUID 4b2a3d4c5e6f708192a3b4c5d6e7f809\nType Tag"),
        ("Bundles/Hat/Hat.dat", "GUID 5b2a3d4c5e6f708192a3b4c5d6e7f809\nType Hat\nID 4"),
        ("Bundles/Hat/English.dat", "Name \"\""),
        ("Bundles/Box/Box.dat", "GUID 6b2a3d4c5e6f708192a3b4c5d6e7f809\nType Large"),
    ]);
    let friendly = |name: &str| loader.headers[index_of(&loader, name)].friendly_name.clone();
    assert_eq!(friendly("Grass_Material"), "Grass");
    assert_eq!(friendly("Road"), "Asphalt");
    assert_eq!(friendly("Tag"), "Tag");
    // An empty item name falls back to the asset's name; an object without a translation gets the key.
    assert_eq!(friendly("Hat"), "Hat");
    assert_eq!(friendly("Box"), "Name");
}
