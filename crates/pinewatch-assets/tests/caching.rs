//! `CachingAssetRef`, `CachingBcAssetRef`, `Assets.find(EAssetType, ushort)` and `UnturnedDatEx`'s readers for
//! them (M2c task 10).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use pinewatch_assets::{
    AssetCategory, AssetLoader, CachingAssetRef, CachingBcAssetRef, EAssetType, Origins, client_search_locations,
    load_client_assets, parse_asset_ref, parse_guid_or_legacy_id_v2, read_location, try_parse_asset_ref,
    try_parse_bc_asset_ref,
};
use pinewatch_dat::{DatParser, DatTree};
use pinewatch_io::{InstallRoot, LogLevel, MemoryLog};
use pinewatch_mono::{ExceptionKind, Guid, NetString};

const HAT: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";
const OTHER: &str = "2b2a3d4c5e6f708192a3b4c5d6e7f809";
const REDIRECT: &str = "3b2a3d4c5e6f708192a3b4c5d6e7f809";
const LOOP_A: &str = "4b2a3d4c5e6f708192a3b4c5d6e7f809";
const LOOP_B: &str = "5b2a3d4c5e6f708192a3b4c5d6e7f809";
const BOX: &str = "8b2a3d4c5e6f708192a3b4c5d6e7f809";
const CORE_CONFIG: &str = "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 6";

fn tree(text: &str) -> DatTree {
    DatParser::new().parse_str(text)
}

fn guid(text: &str) -> Guid {
    Guid::try_parse(Some(&NetString::from(text))).unwrap()
}

fn item() -> AssetCategory {
    AssetCategory::from(EAssetType::Item)
}

fn none() -> AssetCategory {
    AssetCategory::from(EAssetType::None)
}

/// `CachingAssetRef.TryParse(string)`: null, empty and `0` are empty and fail; anything `Guid.TryParse` reads
/// succeeds, the all-zero GUID too.
#[test]
fn caching_asset_ref_from_text() {
    let parse = |text: Option<&str>| CachingAssetRef::try_parse_str(text.map(NetString::from).as_ref());
    assert_eq!(parse(Some(HAT)), Some(CachingAssetRef::new(guid(HAT))));
    for text in [None, Some(""), Some("0"), Some("nonsense"), Some("5")] {
        assert_eq!(parse(text), None, "{text:?}");
    }
    let zero = parse(Some("00000000000000000000000000000000")).expect("Guid.TryParse reads it");
    assert!(zero.is_empty() && !zero.is_assigned());
    assert_eq!(CachingAssetRef::parse_str(Some(&NetString::from("0"))), CachingAssetRef::EMPTY);
}

#[test]
fn caching_asset_ref_from_nodes() {
    let tree = tree(&format!("A {HAT}\nB\n{{\n\tGUID {HAT}\n}}\nC\n{{\n\tID 5\n}}\nD\n[\n]\nE 0"));
    let root = tree.root();
    for key in ["A", "B"] {
        let parsed = root.try_parse_struct::<CachingAssetRef>(key, &mut ()).unwrap();
        assert_eq!(parsed, (true, CachingAssetRef::new(guid(HAT))));
        assert_eq!(try_parse_asset_ref(&root, key), Some(CachingAssetRef::new(guid(HAT))));
    }
    for key in ["C", "D", "E", "Missing"] {
        assert_eq!(root.try_parse_struct::<CachingAssetRef>(key, &mut ()).unwrap(), (false, CachingAssetRef::EMPTY));
        assert_eq!(try_parse_asset_ref(&root, key), None, "{key}");
        assert_eq!(parse_asset_ref(&root, key), CachingAssetRef::EMPTY);
    }
}

/// `CachingBcAssetRef.TryParse(string, defaultLegacyType)`.
#[test]
fn caching_bc_asset_ref_from_text() {
    let parse = |text: &str, default: EAssetType| {
        CachingBcAssetRef::try_parse_str(Some(&NetString::from(text)), AssetCategory::from(default))
    };
    let legacy = |category: AssetCategory, id: u16| CachingBcAssetRef::from_legacy(category, id);
    let empty = CachingBcAssetRef::EMPTY;

    // `type:id`, the type ignoring case (and trimmed), the ID an integer.
    assert_eq!(parse("Item:5", EAssetType::None), (true, legacy(item(), 5)));
    assert_eq!(parse("item: 5", EAssetType::None), (true, legacy(item(), 5)));
    assert_eq!(parse("42:5", EAssetType::None), (true, legacy(AssetCategory(42), 5)));
    // `NONE` refers to nothing but parses; so does ID 0, which drops the type.
    assert_eq!(parse("NONE:5", EAssetType::Item), (true, empty));
    assert_eq!(parse("Item:0", EAssetType::None), (true, empty));
    assert_eq!(parse("Item:0", EAssetType::None).1.legacy_type(), none());
    for text in ["Nonsense:5", "Item:abc", "Item:70000", "Item:-1", "0", "", ":5"] {
        assert_eq!(parse(text, EAssetType::Item), (false, empty), "{text}");
    }
    // Without `:`, a number is a legacy ID only with a default type; otherwise, a GUID.
    assert_eq!(parse("5", EAssetType::Item), (true, legacy(item(), 5)));
    assert_eq!(parse("5", EAssetType::None), (false, empty));
    assert_eq!(parse(HAT, EAssetType::Item), (true, CachingBcAssetRef::from_guid(guid(HAT))));
    assert_eq!(CachingBcAssetRef::try_parse_str(None, item()), (false, empty));
}

/// The node forms: a value as text; a dictionary's `GUID`, else its `Type` and `ID` (where `NONE` isn't special).
#[test]
fn caching_bc_asset_ref_from_nodes() {
    let tree = tree(&format!(
        "Value Item:5\nNumber 5\nGuid\n{{\n\tGUID {HAT}\n}}\nLegacy\n{{\n\tType item\n\tID 7\n}}\n\
         NoneType\n{{\n\tType NONE\n\tID 7\n}}\nNoId\n{{\n\tType ITEM\n\tID nope\n}}\nList\n[\n]"
    ));
    let root = tree.root();
    let v2 = |key: &str| parse_guid_or_legacy_id_v2(&root, key, item());
    assert_eq!(v2("Value"), CachingBcAssetRef::from_legacy(item(), 5));
    assert_eq!(v2("Number"), CachingBcAssetRef::from_legacy(item(), 5));
    assert_eq!(v2("Guid"), CachingBcAssetRef::from_guid(guid(HAT)));
    assert_eq!(v2("Legacy"), CachingBcAssetRef::from_legacy(item(), 7));
    let none_type = v2("NoneType");
    assert!(none_type.is_assigned() && none_type.legacy_type() == none() && none_type.legacy_id() == 7);
    for key in ["NoId", "List", "Missing"] {
        assert_eq!(v2(key), CachingBcAssetRef::EMPTY, "{key}");
        assert_eq!(try_parse_bc_asset_ref(&root, key, item()), None, "{key}");
    }
    assert_eq!(try_parse_bc_asset_ref(&root, "Legacy", item()), Some(CachingBcAssetRef::from_legacy(item(), 7)));

    // `IDatParseable.TryParse` has no default type, and a failed dictionary keeps what it parsed before failing.
    let number = root.try_parse_struct::<CachingBcAssetRef>("Number", &mut ()).unwrap();
    assert_eq!(number, (false, CachingBcAssetRef::EMPTY));
    let (ok, partial) = root.try_parse_struct::<CachingBcAssetRef>("NoId", &mut ()).unwrap();
    assert!(!ok);
    assert_eq!((partial.legacy_type(), partial.legacy_id(), partial.guid()), (item(), 0, Guid::EMPTY));
    assert_ne!(partial, CachingBcAssetRef::EMPTY);
}

/// The static parser builds through the constructor, which drops the type for ID 0; the instance parser writes the
/// fields as parsed.
#[test]
fn a_zero_legacy_id_keeps_its_type_only_through_the_instance_parser() {
    let tree = tree("ZeroId\n{\n\tType ITEM\n\tID 0\n}");
    let root = tree.root();
    assert_eq!(parse_guid_or_legacy_id_v2(&root, "ZeroId", none()), CachingBcAssetRef::EMPTY);
    let (ok, parsed) = root.try_parse_struct::<CachingBcAssetRef>("ZeroId", &mut ()).unwrap();
    assert!(ok);
    assert_eq!((parsed.legacy_type(), parsed.legacy_id()), (item(), 0));
    assert!(parsed.is_empty());
}

/// The constructors' rules: a legacy ID of 0 drops the type, and a legacy ID drops the GUID.
#[test]
fn caching_bc_asset_ref_constructors() {
    let both = CachingBcAssetRef::new(guid(HAT), item(), 5);
    assert_eq!((both.guid(), both.legacy_type(), both.legacy_id()), (Guid::EMPTY, item(), 5));
    let guid_only = CachingBcAssetRef::new(guid(HAT), item(), 0);
    assert_eq!((guid_only.guid(), guid_only.legacy_type()), (guid(HAT), none()));
    let converted = CachingBcAssetRef::from(CachingAssetRef::new(guid(HAT)));
    assert_eq!(converted, CachingBcAssetRef::from_guid(guid(HAT)));
    assert!(CachingBcAssetRef::EMPTY.is_empty() && !CachingBcAssetRef::EMPTY.is_assigned());
}

struct TempInstall(PathBuf);

impl Drop for TempInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A hat (`HAT`, legacy ID 5), another hat (`OTHER`, ID 6), a redirector with item ID 7 to the first hat, two item
/// redirectors (IDs 8 and 9) that point at each other, and an object with ID 6.
fn loaded() -> (TempInstall, AssetLoader) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-caching-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let files = [
        ("MasterBundle.dat", CORE_CONFIG.to_string()),
        ("core.masterbundle", String::new()),
        ("Hat/Hat.dat", format!("GUID {HAT}\nType Hat\nID 5")),
        ("Cap/Cap.dat", format!("GUID {OTHER}\nType Hat\nID 6")),
        ("R/R.dat", format!("GUID {REDIRECT}\nType Redirector\nID 7\nAssetCategory ITEM\nTargetAsset {HAT}")),
        ("A/A.dat", format!("GUID {LOOP_A}\nType Redirector\nID 8\nAssetCategory ITEM\nTargetAsset {LOOP_B}")),
        ("B/B.dat", format!("GUID {LOOP_B}\nType Redirector\nID 9\nAssetCategory ITEM\nTargetAsset {LOOP_A}")),
        ("Box/Box.dat", format!("GUID {BOX}\nType Large\nID 6")),
    ];
    for (file, contents) in &files {
        let path = root.join("Bundles").join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(&root).unwrap();
    let loader = load_client_assets(&install, "English", &mut MemoryLog::default()).unwrap();
    (TempInstall(root), loader)
}

fn index_of(loader: &AssetLoader, name: &str) -> usize {
    loader.headers.iter().position(|header| header.internal_name == name).unwrap()
}

#[test]
fn legacy_find_follows_redirectors() {
    let (_dir, loader) = loaded();
    let hat = index_of(&loader, "Hat");
    let mut log = MemoryLog::default();
    assert_eq!(loader.find_legacy(item(), 5, &mut log).unwrap(), Some(hat));
    assert_eq!(loader.find_legacy(item(), 7, &mut log).unwrap(), Some(hat));
    assert_eq!(loader.find_legacy(item(), 4, &mut log).unwrap(), None);
    // `NONE` and ID 0 find nothing before any lookup; a type the tables don't hold throws.
    assert_eq!(loader.find_legacy(none(), 5, &mut log).unwrap(), None);
    assert_eq!(loader.find_legacy(AssetCategory(42), 0, &mut log).unwrap(), None);
    let error = loader.find_legacy(AssetCategory(42), 5, &mut log).unwrap_err();
    assert_eq!(error.kind, ExceptionKind::KeyNotFound);
    assert_eq!(error.message, "The given key '42' was not present in the dictionary.");
    assert!(log.entries.is_empty(), "{:?}", log.entries);

    assert_eq!(loader.find_legacy(item(), 8, &mut log).unwrap(), None);
    let expected = "Infinite asset director loop encountered when resolving Type: ITEM Legacy ID: 8";
    assert_eq!(log.entries, [(LogLevel::Warn, expected.to_string())]);
}

/// `Get` looks up by GUID, or by legacy type and ID when the ID is set, and keeps what it found.
#[test]
fn get_finds_and_caches() {
    let (_dir, loader) = loaded();
    let (hat, cap) = (index_of(&loader, "Hat"), index_of(&loader, "Cap"));
    let mut log = MemoryLog::default();

    let mut by_guid = CachingAssetRef::new(guid(HAT));
    assert_eq!(by_guid.get(&loader, &mut log), Some(hat));
    assert_eq!(by_guid.cached(), Some(hat));
    // The cache doesn't take part in equality.
    assert_eq!(by_guid, CachingAssetRef::new(guid(HAT)));
    assert_eq!(CachingAssetRef::new(guid(REDIRECT)).get(&loader, &mut log), Some(hat));

    let mut by_id = CachingBcAssetRef::from_legacy(item(), 6);
    assert_eq!(by_id.get(&loader, &mut log).unwrap(), Some(cap));
    assert_eq!(CachingBcAssetRef::from_guid(guid(OTHER)).get(&loader, &mut log).unwrap(), Some(cap));
    assert_eq!(CachingBcAssetRef::from_legacy(none(), 6).get(&loader, &mut log).unwrap(), None);
    let mut undefined = CachingBcAssetRef::from_legacy(AssetCategory(42), 6);
    assert_eq!(undefined.get(&loader, &mut log).unwrap_err().kind, ExceptionKind::KeyNotFound);
    assert!(log.entries.is_empty(), "{:?}", log.entries);

    let mut cleared = by_id;
    cleared.clear();
    assert_eq!(cleared, CachingBcAssetRef::EMPTY);
    assert_eq!(cleared.cached(), None);
}

/// `Get` looks again once its cached asset has been replaced, here by a `Sandbox` asset with the same GUID and ID;
/// until then `IsReferenceTo` compares the cached asset, unless the asset asked about is the replaced one.
#[test]
fn a_replaced_asset_is_looked_up_again() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("pinewatch-assets-replaced-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dir = TempInstall(root.clone());
    for (file, contents) in [
        ("Bundles/MasterBundle.dat", CORE_CONFIG.to_string()),
        ("Bundles/core.masterbundle", String::new()),
        ("Bundles/Hat/Hat.dat", format!("GUID {HAT}\nType Hat\nID 5")),
        ("Sandbox/Mod/Hat2/Hat2.dat", format!("GUID {HAT}\nType Hat\nID 5")),
    ] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(&dir.0).unwrap();
    let mut origins = Origins::new();
    let mut log = MemoryLog::default();
    let locations = client_search_locations(&install, &mut origins, &mut log).unwrap();
    assert_eq!(locations.len(), 2);
    let mut loader = AssetLoader::new(origins);
    loader.take_results(read_location(&install, &locations[0], "English").unwrap(), &mut log);

    let mut by_guid = CachingAssetRef::new(guid(HAT));
    let mut by_id = CachingBcAssetRef::from_legacy(item(), 5);
    let core = by_guid.get(&loader, &mut log).unwrap();
    assert_eq!(by_id.get(&loader, &mut log).unwrap(), Some(core));

    loader.take_results(read_location(&install, &locations[1], "English").unwrap(), &mut log);
    let sandbox = index_of(&loader, "Hat2");
    assert!(loader.headers[core].has_been_replaced);
    assert!(by_guid.is_reference_to(&loader, Some(core)), "a replaced asset is compared by GUID");
    assert!(!by_guid.is_reference_to(&loader, Some(sandbox)), "the cache still holds the core asset");
    assert_eq!(by_guid.get(&loader, &mut log), Some(sandbox));
    assert!(by_guid.is_reference_to(&loader, Some(sandbox)));
    assert_eq!(by_id.get(&loader, &mut log).unwrap(), Some(sandbox));
    // With the new asset cached, the replaced one still matches by GUID, or by type and ID.
    assert!(by_guid.is_reference_to(&loader, Some(core)));
    assert!(by_id.is_reference_to(&loader, Some(core)));
}

/// `IsReferenceTo`: the cached asset when there is one, else the GUID, else the legacy type and ID.
#[test]
fn is_reference_to() {
    let (_dir, loader) = loaded();
    let (hat, cap, redirect) = (index_of(&loader, "Hat"), index_of(&loader, "Cap"), index_of(&loader, "R"));
    let mut log = MemoryLog::default();

    let mut through_redirector = CachingAssetRef::new(guid(REDIRECT));
    assert!(through_redirector.is_reference_to(&loader, Some(redirect)));
    assert!(!through_redirector.is_reference_to(&loader, Some(hat)));
    through_redirector.get(&loader, &mut log);
    assert!(through_redirector.is_reference_to(&loader, Some(hat)));
    assert!(!through_redirector.is_reference_to(&loader, None));
    assert!(CachingAssetRef::EMPTY.is_reference_to(&loader, None));

    let by_id = CachingBcAssetRef::from_legacy(item(), 6);
    assert!(by_id.is_reference_to(&loader, Some(cap)));
    assert!(!by_id.is_reference_to(&loader, Some(hat)));
    // Same ID, other category.
    assert!(!by_id.is_reference_to(&loader, Some(index_of(&loader, "Box"))));
    assert!(!CachingBcAssetRef::EMPTY.is_reference_to(&loader, Some(hat)));
    assert!(CachingBcAssetRef::EMPTY.is_reference_to(&loader, None));
    assert!(!by_id.is_reference_to(&loader, None));
}

/// `ToString`, which looks the asset up for its friendly name.
#[test]
fn to_string_names_the_asset() {
    let (_dir, loader) = loaded();
    let mut log = MemoryLog::default();
    assert_eq!(CachingAssetRef::new(guid(HAT)).format(&loader, &mut log), format!("(ID: {HAT}, Asset: Hat)"));
    let missing = "6b2a3d4c5e6f708192a3b4c5d6e7f809";
    assert_eq!(CachingAssetRef::new(guid(missing)).format(&loader, &mut log), format!("(ID: {missing}, Asset: null)"));
    assert_eq!(
        CachingBcAssetRef::from_guid(guid(OTHER)).format(&loader, &mut log).unwrap(),
        format!("(GUID: {OTHER}, Asset: Cap)")
    );
    assert_eq!(
        CachingBcAssetRef::from_legacy(item(), 7).format(&loader, &mut log).unwrap(),
        "(Legacy Type: ITEM, Legacy ID: 7, Asset: Hat)"
    );
    assert_eq!(
        CachingBcAssetRef::from_legacy(AssetCategory(42), 0).format(&loader, &mut log).unwrap(),
        format!("(GUID: {}, Asset: null)", Guid::EMPTY.to_string_n())
    );
}
