//! Asset and content references, and `UnturnedDatEx`'s GUID-or-legacy-ID helpers (M2c task 10).

use pinewatch_assets::{AssetReference, ContentReference, parse_guid_or_legacy_id, read_asset_reference};
use pinewatch_dat::{DatParser, DatTree};
use pinewatch_mono::{Guid, NetString};

const GUID: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";

fn tree(text: &str) -> DatTree {
    DatParser::new().parse_str(text)
}

fn guid() -> Guid {
    Guid::try_parse(Some(&NetString::from(GUID))).unwrap()
}

fn text(value: &Option<NetString>) -> Option<String> {
    value.as_ref().map(NetString::to_string_lossy)
}

#[test]
fn asset_reference_from_a_value_or_a_dictionary() {
    let tree = tree(&format!("A {GUID}\nB\n{{\n\tGUID {GUID}\n}}\nC nonsense\nD\n[\n]"));
    let root = tree.root();
    assert_eq!(root.try_parse_struct::<AssetReference>("A", &mut ()).unwrap(), (true, AssetReference { guid: guid() }));
    assert_eq!(root.try_parse_struct::<AssetReference>("B", &mut ()).unwrap(), (true, AssetReference { guid: guid() }));
    assert_eq!(root.try_parse_struct::<AssetReference>("C", &mut ()).unwrap(), (false, AssetReference::default()));
    assert_eq!(root.try_parse_struct::<AssetReference>("D", &mut ()).unwrap(), (false, AssetReference::default()));
    assert!(!AssetReference::default().is_valid());
    assert!(AssetReference { guid: guid() }.is_valid());
}

/// `name:path` splits at the first `:`; without one the name is the current master bundle's. Values shorter than
/// two characters are refused (a stray `}` from a common typo), and so are null and empty ones.
#[test]
fn content_reference_forms() {
    let tree = tree("Split core.masterbundle:Items/Gun:A.prefab\nBare Items/Gun.prefab\nShort }\nEmpty \"\"\nFlag\nDict\n{\n\tName core.masterbundle\n\tPath Items/Gun.prefab\n}\nList\n[\n]");
    let root = tree.root();
    let mut with_bundle = Some("core.masterbundle".to_string());
    let mut without_bundle: Option<String> = None;

    let (ok, split) = root.try_parse_struct::<ContentReference>("Split", &mut without_bundle).unwrap();
    assert!(ok);
    assert_eq!(text(&split.name).as_deref(), Some("core.masterbundle"));
    assert_eq!(text(&split.path).as_deref(), Some("Items/Gun:A.prefab"));

    let (ok, bare) = root.try_parse_struct::<ContentReference>("Bare", &mut with_bundle).unwrap();
    assert!(ok && bare.is_valid());
    assert_eq!(text(&bare.name).as_deref(), Some("core.masterbundle"));
    assert_eq!(text(&bare.path).as_deref(), Some("Items/Gun.prefab"));

    let (ok, bare) = root.try_parse_struct::<ContentReference>("Bare", &mut without_bundle).unwrap();
    assert!(ok && !bare.is_valid());
    assert_eq!(bare.name, None);

    for key in ["Short", "Empty", "Flag", "List", "Missing"] {
        let (ok, _) = root.try_parse_struct::<ContentReference>(key, &mut with_bundle).unwrap();
        assert!(!ok, "{key}");
    }

    let (ok, dict) = root.try_parse_struct::<ContentReference>("Dict", &mut without_bundle).unwrap();
    assert!(ok);
    assert_eq!(text(&dict.name).as_deref(), Some("core.masterbundle"));
    assert_eq!(text(&dict.path).as_deref(), Some("Items/Gun.prefab"));
}

/// `ParseGuidOrLegacyId`: a `ushort` first (`NumberStyles.Any`, invariant culture), else a GUID; `0`, empty, null
/// or missing give neither.
#[test]
fn guid_or_legacy_id() {
    let tree = tree(&format!("Id 42\nGrouped \"1,000\"\nZero 0\nBig 70000\nGuid {GUID}\nFlag\nEmpty \"\"\nJunk abc"));
    let root = tree.root();
    assert_eq!(parse_guid_or_legacy_id(&root, "Id"), (Guid::EMPTY, 42));
    assert_eq!(parse_guid_or_legacy_id(&root, "Grouped"), (Guid::EMPTY, 1000));
    assert_eq!(parse_guid_or_legacy_id(&root, "Guid"), (guid(), 0));
    for key in ["Zero", "Big", "Flag", "Empty", "Junk", "Missing"] {
        assert_eq!(parse_guid_or_legacy_id(&root, key), (Guid::EMPTY, 0), "{key}");
    }
}

/// `readAssetReference`: `ParseGuid` when the key exists (`Guid.Empty` if it doesn't parse), else the default.
#[test]
fn read_asset_reference_uses_the_default_only_for_a_missing_key() {
    let tree = tree(&format!("Good {GUID}\nBad nonsense"));
    let root = tree.root();
    let default = AssetReference { guid: guid() };
    assert_eq!(read_asset_reference(&root, "Good", AssetReference::default()), AssetReference { guid: guid() });
    assert_eq!(read_asset_reference(&root, "Bad", default), AssetReference::default());
    assert_eq!(read_asset_reference(&root, "Missing", default), default);
}
