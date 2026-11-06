//! Every `.dat` and `.asset` file in the installed game parses without a panic, with one reused parser as an asset
//! worker uses, with metadata off and on (`AssetsWorker` sets `EnableMetadata` from `Assets.shouldParseMetadata`).
//! The two modes can report different line numbers for the same file, as in the C#, so their errors aren't
//! compared. The walk covers `Bundles/`, `Localization/`, and each map's root `.dat` files (the language files that
//! `LevelInfo.getLocalization` reads through `ReadWrite.ReadDataWithoutHash`) and `Bundles/` folder. It skips each
//! map's `Level.dat`, which is binary (`Level.ReadLevelInfo` reads it with a `Block`). The install is only read. The
//! test is skipped when no install is found.

use pinewatch_dat::DatParser;
use pinewatch_io::{InstallRoot, find_install};

/// Appends the `.dat` and `.asset` files under `dir` (install-relative) to `files`, depth first in name order.
fn dat_files(install: &InstallRoot, dir: &str, files: &mut Vec<String>) {
    let entries = install.read_dir(dir).unwrap_or_else(|error| panic!("{dir}: {error}"));
    let mut names: Vec<String> = entries.into_iter().filter_map(|name| name.into_string().ok()).collect();
    names.sort();
    for name in names {
        let path = format!("{dir}/{name}");
        if install.is_dir(&path) {
            dat_files(install, &path, files);
        } else {
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".dat") || lower.ends_with(".asset") {
                files.push(path);
            }
        }
    }
}

/// Appends each map's root `.dat` files (for example `Maps/PEI/English.dat`) except the binary `Level.dat`, and the
/// `.dat` and `.asset` files under its `Bundles/` folder, maps in name order.
fn map_files(install: &InstallRoot, files: &mut Vec<String>) {
    let entries = install.read_dir("Maps").unwrap_or_else(|error| panic!("Maps: {error}"));
    let mut maps: Vec<String> = entries.into_iter().filter_map(|name| name.into_string().ok()).collect();
    maps.sort();
    for map in maps {
        let map_dir = format!("Maps/{map}");
        if !install.is_dir(&map_dir) {
            continue;
        }
        let entries = install.read_dir(&map_dir).unwrap_or_else(|error| panic!("{map_dir}: {error}"));
        let mut names: Vec<String> = entries.into_iter().filter_map(|name| name.into_string().ok()).collect();
        names.sort();
        for name in names {
            let path = format!("{map_dir}/{name}");
            if install.is_dir(&path) {
                if name == "Bundles" {
                    dat_files(install, &path, files);
                }
            } else if name.to_ascii_lowercase().ends_with(".dat") && !name.eq_ignore_ascii_case("Level.dat") {
                files.push(path);
            }
        }
    }
}

/// Every file the test parses, and how many of them came from `Maps/`.
fn installed_files(install: &InstallRoot) -> (Vec<String>, usize) {
    let mut files = Vec::new();
    for dir in ["Bundles", "Localization"] {
        dat_files(install, dir, &mut files);
    }
    let before_maps = files.len();
    map_files(install, &mut files);
    let map_count = files.len() - before_maps;
    (files, map_count)
}

/// The walk on a small made-up install tree, so it is checked even where no game is installed.
#[test]
fn walk_includes_map_language_files_and_map_bundles() {
    let dir = std::env::temp_dir().join(format!("pinewatch-dat-walk-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, contents) in [
        ("Bundles/Items/A/A.dat", "ID 1"),
        ("Localization/English/Menu.dat", "Key Value"),
        ("Maps/PEI/English.dat", "Name PEI"),
        ("Maps/PEI/Config.json", "{}"),
        ("Maps/PEI/Level.dat", "binary, not parsed"),
        ("Maps/PEI/Level.hierarchy", ""),
        ("Maps/PEI/Bundles/Spawns/S.asset", "GUID 0"),
        ("Maps/PEI/Spawns/Players.dat", "binary, not parsed"),
        ("Maps/Washington/LEVEL.DAT", "binary, not parsed"),
        ("Maps/Washington/Russian.dat", "Name W"),
    ] {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, contents).unwrap();
    }
    let install = InstallRoot::open(&dir).unwrap();
    let (files, map_count) = installed_files(&install);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        files,
        [
            "Bundles/Items/A/A.dat",
            "Localization/English/Menu.dat",
            "Maps/PEI/Bundles/Spawns/S.asset",
            "Maps/PEI/English.dat",
            "Maps/Washington/Russian.dat",
        ]
    );
    assert_eq!(map_count, 3);
}

#[test]
fn every_installed_dat_and_asset_file_parses() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };

    let (files, map_count) = installed_files(&install);

    let mut parser = DatParser::new();
    let mut metadata_parser = DatParser::new();
    metadata_parser.set_enable_metadata(true);
    let mut with_errors = 0;
    let mut samples = Vec::new();
    for file in &files {
        let bytes = install.read(file).unwrap_or_else(|error| panic!("{file}: {error}"));
        parser.parse_bytes(&bytes);
        metadata_parser.parse_bytes(&bytes);
        if parser.has_error() {
            with_errors += 1;
            if samples.len() < 5 {
                samples.push(format!("{file}: {}", parser.error_messages()[0]));
            }
        }
    }

    eprintln!("parsed {} files ({map_count} from Maps/), {with_errors} with errors", files.len());
    for sample in &samples {
        eprintln!("  {sample}");
    }
    assert!(map_count > 0, "no map .dat files found under Maps/");
    assert!(
        files.iter().any(|file| file.starts_with("Maps/") && file.ends_with("/English.dat")),
        "no Maps/*/English.dat found"
    );
    assert!(files.len() >= 8000, "only {} .dat and .asset files found", files.len());
}
