//! `AssetsWorker`'s searcher: the breadth-first walk of a search location, `MasterBundle.dat` detection and
//! `FindAssets`' file precedence (M2c task 3).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{MonoPath, SearchEvent, WorkItem, search_location};
use pinewatch_io::InstallRoot;
use pinewatch_mono::ExceptionKind;

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_install(files: &[&str]) -> (TempInstall, InstallRoot) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-walk-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Bundles")).unwrap();
    for file in files {
        let path = root.join(file);
        if file.ends_with('/') {
            fs::create_dir_all(path).unwrap();
        } else {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"").unwrap();
        }
    }
    let install = InstallRoot::open(&root).unwrap();
    (TempInstall(root), install)
}

fn bundles(install: &InstallRoot) -> MonoPath {
    let separator = if cfg!(windows) { "\\" } else { "/" };
    MonoPath { full: format!("{}{separator}Bundles", install.full_name()), rel: "Bundles".to_string() }
}

/// Each event as `kind:rel`, or `exception:message` for an exception event.
fn summary(events: &[SearchEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| match event {
            SearchEvent::Item(WorkItem::MasterBundle(path)) => format!("master bundle:{}", path.rel),
            SearchEvent::Item(WorkItem::Asset(path)) => format!("asset:{}", path.rel),
            SearchEvent::Exception { message, .. } => format!("exception:{message}"),
        })
        .collect()
}

#[test]
fn folders_are_searched_breadth_first() {
    let (_dir, install) = temp_install(&[
        "Bundles/MasterBundle.dat",
        "Bundles/Items/Items.dat",
        "Bundles/Items/Gun/Gun.dat",
        "Bundles/Items/Gun/Scope/Scope.dat",
    ]);
    let events = search_location(&install, &bundles(&install)).unwrap();
    assert_eq!(summary(&events), [
        "master bundle:Bundles/MasterBundle.dat",
        "asset:Bundles/Items/Items.dat",
        "asset:Bundles/Items/Gun/Gun.dat",
        "asset:Bundles/Items/Gun/Scope/Scope.dat",
    ]);
}

#[test]
fn the_whole_first_level_comes_before_the_second() {
    let (_dir, install) = temp_install(&["Bundles/A/A.dat", "Bundles/A/Deep/Deep.dat", "Bundles/B/B.dat"]);
    let events = summary(&search_location(&install, &bundles(&install)).unwrap());
    let position = |name: &str| events.iter().position(|event| event == name).unwrap();
    assert!(position("asset:Bundles/A/A.dat") < position("asset:Bundles/A/Deep/Deep.dat"));
    assert!(position("asset:Bundles/B/B.dat") < position("asset:Bundles/A/Deep/Deep.dat"));
}

/// `<Folder>.asset`, else `<Folder>.dat`, else `Asset.dat`, else every `*.asset`; the first match ends the search
/// in that folder.
#[test]
fn file_precedence() {
    let (_dir, install) = temp_install(&[
        "Bundles/One/One.asset",
        "Bundles/One/One.dat",
        "Bundles/One/Asset.dat",
        "Bundles/One/Other.asset",
        "Bundles/Two/Two.dat",
        "Bundles/Two/Asset.dat",
        "Bundles/Two/Other.asset",
        "Bundles/Three/Asset.dat",
        "Bundles/Three/Other.asset",
        "Bundles/Four/Other.asset",
        "Bundles/Four/Notes.txt",
        "Bundles/Four/Four.txt",
    ]);
    let mut events = summary(&search_location(&install, &bundles(&install)).unwrap());
    events.sort();
    assert_eq!(events, [
        "asset:Bundles/Four/Other.asset",
        "asset:Bundles/One/One.asset",
        "asset:Bundles/Three/Asset.dat",
        "asset:Bundles/Two/Two.dat",
    ]);
}

/// Every `*.asset` file of a folder, in enumeration order. A folder named like an asset file doesn't count.
#[test]
fn every_asset_file_in_enumeration_order() {
    let (_dir, install) = temp_install(&[
        "Bundles/Pack/c.asset",
        "Bundles/Pack/a.asset",
        "Bundles/Pack/b.asset",
        "Bundles/Pack/x.asset.bak",
        "Bundles/Pack/Folder.asset/",
    ]);
    let events = summary(&search_location(&install, &bundles(&install)).unwrap());
    let expected: Vec<String> = install
        .read_dir("Bundles/Pack")
        .unwrap()
        .into_iter()
        .map(|name| name.into_string().unwrap())
        .filter(|name| name.ends_with(".asset") && name != "Folder.asset")
        .map(|name| format!("asset:Bundles/Pack/{name}"))
        .collect();
    assert_eq!(events[..3], expected[..]);
    assert_eq!(events.len(), 3);
}

/// `*.asset` ignores case on Windows and respects it on Linux, as corefx's platform default does.
#[test]
fn asset_extension_case_follows_the_platform() {
    let (_dir, install) = temp_install(&["Bundles/Pack/Upper.ASSET"]);
    let events = search_location(&install, &bundles(&install)).unwrap();
    assert_eq!(events.len(), usize::from(cfg!(windows)));
}

/// A location that doesn't exist: `File.Exists` is false, and both enumerations throw.
#[test]
fn missing_location_reports_both_enumerations() {
    let (_dir, install) = temp_install(&[]);
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let missing = MonoPath { full: format!("{}{separator}Gone", install.full_name()), rel: "Gone".to_string() };
    let events = search_location(&install, &missing).unwrap();
    assert_eq!(summary(&events), [
        format!("exception:Caught exception checking for assets in: \"{}\"", missing.full),
        format!("exception:Caught exception finding asset subdirectories in: \"{}\"", missing.full),
    ]);
    let SearchEvent::Exception { exception, .. } = &events[0] else { unreachable!() };
    assert_eq!(exception.kind, ExceptionKind::DirectoryNotFound);
    assert_eq!(exception.message, format!("Could not find a part of the path '{}'.", missing.full));
}

/// Paths are the Mono strings: `Path.Combine` for the files it tests, `Path.Join` for enumerated entries.
#[test]
fn work_items_carry_the_game_path() {
    let (_dir, install) = temp_install(&["Bundles/Items/Items.dat", "Bundles/Items/Gun/Gun.asset"]);
    let root = bundles(&install);
    let events = search_location(&install, &root).unwrap();
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let fulls: Vec<&str> = events
        .iter()
        .map(|event| match event {
            SearchEvent::Item(WorkItem::Asset(path)) => path.full.as_str(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(fulls, [
        format!("{}{separator}Items{separator}Items.dat", root.full),
        format!("{}{separator}Items{separator}Gun{separator}Gun.asset", root.full),
    ]);
}
