//! `Assets.find`, which follows redirectors, its typed form for landscape materials, and loading every client search
//! location (M2c task 11).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{AssetLoader, AssetReference, load_client_assets};
use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};
use pinewatch_mono::{Guid, NetString};

const CORE_CONFIG: &str = "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 6";
const MATERIAL: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";
const ITEM: &str = "2b2a3d4c5e6f708192a3b4c5d6e7f809";

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An install with the core master bundle and `files` under `Bundles`, loaded the way the client loads assets.
fn load(files: &[(String, String)]) -> (TempInstall, AssetLoader, MemoryLog) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-find-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mut all = vec![
        ("MasterBundle.dat".to_string(), CORE_CONFIG.to_string()),
        ("core.masterbundle".to_string(), String::new()),
    ];
    all.extend_from_slice(files);
    for (file, contents) in &all {
        let path = root.join("Bundles").join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(&root).unwrap();
    let mut log = MemoryLog::default();
    let loader = load_client_assets(&install, "English", &mut log).unwrap();
    (TempInstall(root), loader, log)
}

fn material(name: &str, guid: &str) -> (String, String) {
    let data = format!(
        "Metadata\n{{\n\tGUID {guid}\n\tType \"SDG.Unturned.LandscapeMaterialAsset, Assembly-CSharp\"\n}}\n\
         Asset\n{{\n\tPhysics_Material SAND_STATIC\n}}"
    );
    (format!("{name}/{name}.asset"), data)
}

fn redirector(name: &str, guid: &str, target: &str) -> (String, String) {
    (format!("{name}/{name}.dat"), format!("GUID {guid}\nType Redirector\nTargetAsset {target}"))
}

fn guid(text: &str) -> Guid {
    Guid::try_parse(Some(&NetString::from(text))).unwrap()
}

/// The `n`th made-up GUID, `000…0n`.
fn numbered(n: usize) -> String {
    format!("{n:032x}")
}

fn index_of(loader: &AssetLoader, name: &str) -> usize {
    loader.headers.iter().position(|header| header.internal_name == name).unwrap()
}

fn warnings(log: &MemoryLog) -> Vec<String> {
    log.entries.iter().filter(|(level, _)| *level == LogLevel::Warn).map(|(_, text)| text.clone()).collect()
}

#[test]
fn find_follows_redirectors_to_their_target() {
    let (_dir, loader, mut log) = load(&[
        material("M", MATERIAL),
        redirector("R1", &numbered(1), MATERIAL),
        redirector("R2", &numbered(2), &numbered(1)),
        redirector("Lost", &numbered(3), &numbered(99)),
    ]);
    let m = index_of(&loader, "M");
    let before = log.entries.len();
    assert_eq!(loader.find(guid(MATERIAL), &mut log), Some(m));
    assert_eq!(loader.find(guid(&numbered(1)), &mut log), Some(m));
    assert_eq!(loader.find(guid(&numbered(2)), &mut log), Some(m));
    // A redirector whose target isn't there finds nothing, without a message; so does an unknown GUID.
    assert_eq!(loader.find(guid(&numbered(3)), &mut log), None);
    assert_eq!(loader.find(guid(&numbered(42)), &mut log), None);
    assert_eq!(loader.find(Guid::EMPTY, &mut log), None);
    assert_eq!(log.entries.len(), before, "{:?}", &log.entries[before..]);
}

/// A chain of 32 redirectors resolves; at the 33rd redirect `find` gives up with a warning naming the GUID asked
/// for (`Guid.ToString()`, the `D` format).
#[test]
fn find_gives_up_after_32_redirects() {
    let mut files = vec![material("M", MATERIAL)];
    // R1 → R2 → … → R33 → M.
    for n in 1..=33 {
        let target = if n == 33 { MATERIAL.to_string() } else { numbered(n + 1) };
        files.push(redirector(&format!("R{n}"), &numbered(n), &target));
    }
    let (_dir, loader, mut log) = load(&files);
    let m = index_of(&loader, "M");
    assert_eq!(loader.find(guid(&numbered(2)), &mut log), Some(m));
    assert!(warnings(&log).is_empty(), "{:?}", log.entries);
    assert_eq!(loader.find(guid(&numbered(1)), &mut log), None);
    let expected = format!("Infinite asset director loop encountered when resolving: {}", guid(&numbered(1)));
    assert_eq!(warnings(&log), [expected]);
    assert!(warnings(&log)[0].ends_with("00000000-0000-0000-0000-000000000001"));
}

#[test]
fn a_redirector_loop_warns() {
    let (_dir, loader, mut log) =
        load(&[redirector("A", &numbered(1), &numbered(2)), redirector("B", &numbered(2), &numbered(1))]);
    assert_eq!(loader.find(guid(&numbered(2)), &mut log), None);
    let expected = format!("Infinite asset director loop encountered when resolving: {}", guid(&numbered(2)));
    assert_eq!(warnings(&log), [expected]);
}

/// `Assets.find<LandscapeMaterialAsset>(reference)`: nothing for an invalid reference, `as T` after `find`.
#[test]
fn landscape_materials_by_reference() {
    let (_dir, loader, mut log) = load(&[
        material("M", MATERIAL),
        redirector("R", &numbered(1), MATERIAL),
        ("Item/Item.dat".to_string(), format!("GUID {ITEM}\nType Hat\nID 5")),
        redirector("ToItem", &numbered(2), ITEM),
    ]);
    let m = index_of(&loader, "M");
    let found = |log: &mut MemoryLog, text: &str| {
        loader.find_landscape_material(AssetReference { guid: guid(text) }, log).map(|(index, _)| index)
    };
    assert_eq!(found(&mut log, MATERIAL), Some(m));
    assert_eq!(found(&mut log, &numbered(1)), Some(m));
    assert_eq!(found(&mut log, ITEM), None);
    assert_eq!(found(&mut log, &numbered(2)), None);
    let (_, asset) = loader.find_landscape_material(AssetReference { guid: guid(MATERIAL) }, &mut log).unwrap();
    assert_eq!(asset.physics_material_name, Some(NetString::from("Sand")));
    assert!(loader.find_landscape_material(AssetReference::default(), &mut log).is_none());
}

/// Every client search location, in order, read and loaded: the same messages as searching by hand.
#[test]
fn client_assets_load_every_location() {
    let (dir, loader, log) = load(&[material("M", MATERIAL)]);
    assert!(loader.core_master_bundle().is_some());
    assert_eq!(loader.headers.len(), 1);
    let install = InstallRoot::open(&dir.0).unwrap();
    let full = format!("{}{}Bundles", install.full_name(), if cfg!(windows) { "\\" } else { "/" });
    let added = format!("Vanilla Built-in Assets added asset search location \"{full}\"");
    assert_eq!(log.entries.first(), Some(&(LogLevel::Info, added)));
}
