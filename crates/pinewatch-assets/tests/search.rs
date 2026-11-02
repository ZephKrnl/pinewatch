//! The client's asset search locations and origins, in `Assets.LoadAllAssets`' order (M2c task 2).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{Origins, client_search_locations};
use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_install(files: &[&str], dirs: &[&str]) -> TempInstall {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-search-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for dir in dirs {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }
    TempInstall(root)
}

/// A path under the install as the game writes it: the root, then the parts with the platform separator.
fn full(root: &Path, parts: &[&str]) -> String {
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let mut path = root.to_str().unwrap().to_string();
    for part in parts {
        path.push_str(separator);
        path.push_str(part);
    }
    path
}

fn info(text: String) -> (LogLevel, String) {
    (LogLevel::Info, text)
}

#[test]
fn core_sandbox_and_map_locations_in_order() {
    let dir = temp_install(&["Maps/PEI/Level.dat", "Maps/Washington/Level.dat"], &[
        "Bundles",
        "Sandbox/Alpha",
        "Maps/PEI/Bundles",
        "Maps/NoLevel/Bundles",
    ]);
    let install = InstallRoot::open(&dir.0).unwrap();
    let mut origins = Origins::new();
    let mut log = MemoryLog::default();
    let locations = client_search_locations(&install, &mut origins, &mut log).unwrap();

    let root = Path::new(install.full_name());
    let paths: Vec<&str> = locations.iter().map(|location| location.dir.full.as_str()).collect();
    assert_eq!(paths, [full(root, &["Bundles"]), full(root, &["Sandbox", "Alpha"]), full(root, &["Maps", "PEI", "Bundles"])]);
    let owners: Vec<&str> = locations.iter().map(|location| origins.get(location.origin).name.as_str()).collect();
    assert_eq!(owners, ["Vanilla Built-in Assets", "Sandbox Folder \"Alpha\"", "Map \"PEI\""]);
    let rels: Vec<&str> = locations.iter().map(|location| location.dir.rel.as_str()).collect();
    assert_eq!(rels, ["Bundles", "Sandbox/Alpha", "Maps/PEI/Bundles"]);

    // Washington is a level without Bundles; NoLevel has no Level.dat. The maps are listed in enumeration order.
    let map_order: Vec<String> = install
        .read_dir("Maps")
        .unwrap()
        .into_iter()
        .map(|name| name.into_string().unwrap())
        .filter(|name| name != "NoLevel")
        .collect();
    let mut expected = vec![
        info(format!("Vanilla Built-in Assets added asset search location \"{}\"", full(root, &["Bundles"]))),
        info("Sandbox: Alpha".to_string()),
        info(format!("Sandbox Folder \"Alpha\" added asset search location \"{}\"", full(root, &["Sandbox", "Alpha"]))),
    ];
    for map in &map_order {
        expected.push(info(format!("Discovered level \"{map}\" at \"{}\"", full(root, &["Maps", map]))));
    }
    expected.push(info(format!("Map \"PEI\" added asset search location \"{}\"", full(root, &["Maps", "PEI", "Bundles"]))));
    assert_eq!(log.entries, expected);
}

#[test]
fn origins_follow_load_all_assets() {
    let dir = temp_install(&["Maps/PEI/Level.dat"], &["Bundles", "Sandbox/Alpha", "Maps/PEI/Bundles"]);
    let install = InstallRoot::open(&dir.0).unwrap();
    let mut origins = Origins::new();
    client_search_locations(&install, &mut origins, &mut MemoryLog::default()).unwrap();
    let all: Vec<(&str, bool, bool, u64)> = origins
        .iter()
        .map(|origin| {
            (
                origin.name.as_str(),
                origin.should_assets_override_existing_ids,
                origin.can_resave,
                origin.workshop_file_id,
            )
        })
        .collect();
    assert_eq!(all, [
        ("Vanilla Built-in Assets", false, false, 0),
        ("Reloaded Assets (Debug)", true, false, 0),
        ("Server Common (Legacy)", false, false, 0),
        ("Per-Server (Legacy)", false, false, 0),
        ("Sandbox Folder \"Alpha\"", true, true, 0),
        ("Map \"PEI\"", false, true, 0),
    ]);
}

/// The original creates a missing `Sandbox` folder; Pinewatch never writes to the install (L3).
#[test]
fn missing_sandbox_is_not_created() {
    let dir = temp_install(&[], &["Bundles", "Maps"]);
    let install = InstallRoot::open(&dir.0).unwrap();
    let locations = client_search_locations(&install, &mut Origins::new(), &mut MemoryLog::default()).unwrap();
    assert_eq!(locations.len(), 1);
    assert!(!dir.0.join("Sandbox").exists());
}

#[test]
fn sandbox_folders_follow_enumeration_order() {
    let dir = temp_install(&[], &["Bundles", "Maps", "Sandbox/C", "Sandbox/A", "Sandbox/B"]);
    fs::write(dir.0.join("Sandbox/file.txt"), b"").unwrap();
    let install = InstallRoot::open(&dir.0).unwrap();
    let mut origins = Origins::new();
    let locations = client_search_locations(&install, &mut origins, &mut MemoryLog::default()).unwrap();
    let expected: Vec<String> = install
        .read_dir("Sandbox")
        .unwrap()
        .into_iter()
        .map(|name| name.into_string().unwrap())
        .filter(|name| name != "file.txt")
        .map(|name| format!("Sandbox Folder \"{name}\""))
        .collect();
    let found: Vec<&str> = locations[1..].iter().map(|location| origins.get(location.origin).name.as_str()).collect();
    assert_eq!(found, expected);
}

/// `Directory.GetDirectories` on a missing `Maps` folder throws `DirectoryNotFoundException`, which
/// `ScanKnownLevels` logs and survives. The stack-trace line is L1.
#[test]
fn missing_maps_folder_is_logged() {
    let dir = temp_install(&[], &["Bundles"]);
    let install = InstallRoot::open(&dir.0).unwrap();
    let mut log = MemoryLog::default();
    let locations = client_search_locations(&install, &mut Origins::new(), &mut log).unwrap();
    assert_eq!(locations.len(), 1);
    let root = Path::new(install.full_name());
    assert_eq!(log.entries[1..], [
        (LogLevel::Error, "Caught exception loading levels in root Maps folder:".to_string()),
        (LogLevel::Error, format!("Could not find a part of the path '{}'.", full(root, &["Maps"]))),
    ]);
}

/// With `WITH_NOREDIST` the core location is added even when `Bundles` is missing (V21).
#[test]
fn core_location_is_added_without_bundles() {
    let dir = temp_install(&[], &["Maps"]);
    let install = InstallRoot::open(&dir.0).unwrap();
    let locations = client_search_locations(&install, &mut Origins::new(), &mut MemoryLog::default()).unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].dir.rel, "Bundles");
}
