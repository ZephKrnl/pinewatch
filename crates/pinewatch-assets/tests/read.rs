//! `AssetsWorker`'s reader: definitions with their hash and parser errors, translations with English fallback
//! (M2c task 4), and master bundle configs (task 5).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{MonoPath, Origins, ResultItem, SearchEvent, SearchLocation, WorkItem, read_items, read_location};
use pinewatch_io::InstallRoot;
use pinewatch_mono::{ExceptionKind, NetString};
use sha1::{Digest, Sha1};

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_install(files: &[(&str, &[u8])]) -> (TempInstall, InstallRoot) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-read-{}-{n}", std::process::id()));
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

const SEPARATOR: &str = if cfg!(windows) { "\\" } else { "/" };

fn location(install: &InstallRoot, origins: &Origins) -> SearchLocation {
    let dir = MonoPath { full: format!("{}{SEPARATOR}Bundles", install.full_name()), rel: "Bundles".to_string() };
    SearchLocation { dir, origin: origins.core() }
}

fn read(install: &InstallRoot, language: &str) -> Vec<ResultItem> {
    let origins = Origins::new();
    read_location(install, &location(install, &origins), language).unwrap()
}

fn string(tree: &pinewatch_dat::DatTree, key: &str) -> Option<String> {
    tree.root().get_string(key, None).map(NetString::to_string_lossy)
}

#[test]
fn definition_with_hash_tree_and_errors() {
    let contents = b"Type Gun\nID 42\nName \"unterminated";
    let (_dir, install) = temp_install(&[("Bundles/Items/Gun/Gun.dat", contents)]);
    let items = read(&install, "English");
    let [ResultItem::Asset(definition)] = &items[..] else { panic!("{items:?}") };
    assert_eq!(definition.path.rel, "Bundles/Items/Gun/Gun.dat");
    let expected_hash: [u8; 20] = Sha1::digest(contents).into();
    assert_eq!(definition.hash, expected_hash);
    assert_eq!(string(&definition.asset_data, "Type").as_deref(), Some("Gun"));
    let errors: Vec<String> = definition.asset_errors.as_ref().unwrap().iter().map(NetString::to_string_lossy).collect();
    assert_eq!(errors, ["missing closing quotation mark (\") for string opened on line 3"]);
    assert!(definition.translation_data.is_none() && definition.fallback_translation_data.is_none());
}

#[test]
fn definition_without_errors_has_none() {
    let (_dir, install) = temp_install(&[("Bundles/Items/Gun/Gun.dat", b"Type Gun")]);
    let items = read(&install, "English");
    let [ResultItem::Asset(definition)] = &items[..] else { panic!("{items:?}") };
    assert!(definition.asset_errors.is_none());
}

/// `<Language>.dat` if it exists, plus `English.dat` as fallback when the language isn't English; otherwise
/// `English.dat` alone.
#[test]
fn translations_and_english_fallback() {
    let files: &[(&str, &[u8])] = &[
        ("Bundles/A/A.dat", b"Type Gun"),
        ("Bundles/A/English.dat", b"Name Rifle"),
        ("Bundles/A/German.dat", b"Name Gewehr"),
    ];
    let (_dir, install) = temp_install(files);
    let cases = [
        ("German", Some("Gewehr"), Some("Rifle")),
        ("English", Some("Rifle"), None),
        ("French", Some("Rifle"), None),
    ];
    for (language, translation, fallback) in cases {
        let items = read(&install, language);
        let [ResultItem::Asset(definition)] = &items[..] else { panic!("{items:?}") };
        let name = |tree: &Option<pinewatch_dat::DatTree>| tree.as_ref().and_then(|tree| string(tree, "Name"));
        assert_eq!(name(&definition.translation_data).as_deref(), translation, "{language}");
        assert_eq!(name(&definition.fallback_translation_data).as_deref(), fallback, "{language}");
    }

    let (_dir, install) = temp_install(&[("Bundles/B/B.dat", b"Type Gun"), ("Bundles/B/German.dat", b"Name Gewehr")]);
    let items = read(&install, "German");
    let [ResultItem::Asset(definition)] = &items[..] else { panic!("{items:?}") };
    assert!(definition.translation_data.is_some() && definition.fallback_translation_data.is_none());
}

/// A definition that can't be read becomes the reader's exception item. The I/O text is Rust's for now (V12).
#[test]
fn unreadable_definition_becomes_an_exception() {
    let (_dir, install) = temp_install(&[]);
    let origins = Origins::new();
    let missing = MonoPath { full: format!("{}{SEPARATOR}Bundles{SEPARATOR}Gone.dat", install.full_name()), rel: "Bundles/Gone.dat".to_string() };
    let events = vec![SearchEvent::Item(WorkItem::Asset(missing.clone()))];
    let items = read_items(&install, origins.core(), events, "English");
    let [ResultItem::Exception { message, exception }] = &items[..] else { panic!("{items:?}") };
    assert_eq!(message, &format!("Caught exception reading asset definition at: \"{}\"", missing.full));
    assert_eq!(exception.kind, ExceptionKind::Io);
}

fn master_bundle_items(config: &str, bundle: Option<&[u8]>) -> (TempInstall, InstallRoot, Vec<ResultItem>) {
    let mut files: Vec<(&str, &[u8])> = vec![("Bundles/MasterBundle.dat", config.as_bytes())];
    if let Some(bundle) = bundle {
        files.push(("Bundles/core.masterbundle", bundle));
    }
    let (dir, install) = temp_install(&files);
    let items = read(&install, "English");
    (dir, install, items)
}

#[test]
fn master_bundle_config_and_hash() {
    let bundle = b"UnityFS fake bundle";
    let (_dir, install, items) = master_bundle_items(
        "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 4\nAsset_Bundle_Version 6",
        Some(bundle),
    );
    let [ResultItem::MasterBundle { config, hash }] = &items[..] else { panic!("{items:?}") };
    assert_eq!(config.asset_bundle_name, "core.masterbundle");
    assert_eq!(config.asset_bundle_name_without_extension, "core");
    assert_eq!(config.asset_prefix, "Assets/Bundles");
    assert_eq!(config.version, 4);
    assert_eq!(config.directory_path.full, format!("{}{SEPARATOR}Bundles", install.full_name()));
    assert_eq!(config.asset_bundle_path().unwrap().rel, "Bundles/core.masterbundle");
    assert!(!config.does_hash_file_exist);
    let expected: [u8; 20] = Sha1::digest(bundle).into();
    assert_eq!(*hash, expected);
}

#[test]
fn master_bundle_version_defaults_and_hash_file() {
    let cases = [
        ("Asset_Bundle_Version 5", 5),
        ("", 2),
        ("Master_Bundle_Version 3\nAsset_Bundle_Version 1", 3),
    ];
    for (extra, version) in cases {
        let config = format!("Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\n{extra}");
        let (dir, _install, items) = master_bundle_items(&config, Some(b""));
        let [ResultItem::MasterBundle { config, .. }] = &items[..] else { panic!("{extra}: {items:?}") };
        assert_eq!(config.version, version, "{extra}");
        drop(dir);
    }
    let (dir, install) = temp_install(&[
        ("Bundles/MasterBundle.dat", b"Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles"),
        ("Bundles/core.masterbundle", b""),
        ("Bundles/core.masterbundle.hash", b""),
    ]);
    let items = read(&install, "English");
    let [ResultItem::MasterBundle { config, .. }] = &items[..] else { panic!("{items:?}") };
    assert!(config.does_hash_file_exist);
    drop(dir);
}

#[test]
fn master_bundle_config_exceptions() {
    let separator = SEPARATOR;
    let cases: [(&str, bool, &str); 6] = [
        (
            "Asset_Prefix Assets/Bundles",
            true,
            "Unspecified Asset_Bundle_Name! This should be the file name and extension of the master asset bundle exported from Unity.",
        ),
        (
            "Asset_Bundle_Name core.masterbundle",
            true,
            "Unspecified Asset_Prefix! This should be the portion of the Unity asset path prior to the /Bundles/ path, e.g. Assets/Bundles/",
        ),
        (
            "Asset_Bundle_Name core.masterbundle\nAsset_Prefix A\nMaster_Bundle_Version 1",
            true,
            "Lowest master bundle version is 2 (default), associated with 2017.4 LTS.",
        ),
        (
            "Asset_Bundle_Name core.masterbundle\nAsset_Prefix A\nMaster_Bundle_Version abc",
            true,
            "Lowest master bundle version is 2 (default), associated with 2017.4 LTS.",
        ),
        (
            "Asset_Bundle_Name core.masterbundle\nAsset_Prefix A\nAsset_Bundle_Version 7",
            true,
            "Highest master bundle version is 6, associated with 2022 LTS.",
        ),
        ("Asset_Bundle_Name core.masterbundle\nAsset_Prefix A", false, "Unable to find specified Asset_Bundle_Name next to the config file! Expected path: "),
    ];
    for (config, with_bundle, expected) in cases {
        let (_dir, install, items) = master_bundle_items(config, with_bundle.then_some(b"".as_slice()));
        let [ResultItem::Exception { message, exception }] = &items[..] else { panic!("{config}: {items:?}") };
        let config_path = format!("{}{separator}Bundles{separator}MasterBundle.dat", install.full_name());
        assert_eq!(message, &format!("Caught exception reading master bundle config at: \"{config_path}\""));
        assert_eq!(exception.kind, ExceptionKind::Exception, "{config}");
        if with_bundle {
            assert_eq!(exception.message, expected, "{config}");
        } else {
            let bundle_path = format!("{}{separator}Bundles{separator}core.masterbundle", install.full_name());
            assert_eq!(exception.message, format!("{expected}{bundle_path}"));
        }
    }
}

#[test]
fn master_bundle_comes_before_the_folder_assets() {
    let (_dir, _install, items) = {
        let (dir, install) = temp_install(&[
            ("Bundles/MasterBundle.dat", b"Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles"),
            ("Bundles/core.masterbundle", b""),
            ("Bundles/Bundles.dat", b"Type Gun"),
        ]);
        let items = read(&install, "English");
        (dir, install, items)
    };
    assert!(matches!(items[..], [ResultItem::MasterBundle { .. }, ResultItem::Asset(_)]), "{items:?}");
}

#[test]
fn format_asset_path() {
    let (_dir, _install, items) =
        master_bundle_items("Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles", Some(b""));
    let [ResultItem::MasterBundle { config, .. }] = &items[..] else { panic!("{items:?}") };
    assert_eq!(config.format_asset_path("Items/Gun/Item.prefab"), "Assets/Bundles/Items/Gun/Item.prefab");
    assert_eq!(config.format_asset_path("/Items/Gun/Item.prefab"), "Assets/Bundles/Items/Gun/Item.prefab");
}
