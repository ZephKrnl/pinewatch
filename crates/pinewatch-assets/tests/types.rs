//! The asset classes: the legacy `Type` keys, `Metadata.Type` names resolved as `Type.GetType` does from
//! `UnturnedDat.dll`, categories and friendly type names (M2c task 7).

use pinewatch_assets::{AssetType, AssetTypeResolver, EAssetType, legacy_asset_type, legacy_keys, resolve_type_name};
use pinewatch_dat::DatParser;

fn legacy(key: &str) -> Option<String> {
    legacy_asset_type(key).map(|t| t.full_name())
}

#[test]
fn legacy_keys_map_to_classes() {
    assert_eq!(legacy("Gun").as_deref(), Some("SDG.Unturned.ItemGunAsset"));
    assert_eq!(legacy("Map").as_deref(), Some("SDG.Unturned.ItemMapAsset"));
    assert_eq!(legacy("Compass").as_deref(), Some("SDG.Unturned.ItemMapAsset"));
    for key in ["Large", "Medium", "Small", "Decal"] {
        assert_eq!(legacy(key).as_deref(), Some("SDG.Unturned.ObjectAsset"), "{key}");
    }
    assert_eq!(legacy("NPC").as_deref(), Some("SDG.Unturned.ObjectNPCAsset"));
    assert_eq!(legacy("RewardsList").as_deref(), Some("SDG.Unturned.NPCRewardsAsset"));
    assert_eq!(legacy("Road").as_deref(), Some("SDG.Unturned.RoadAsset"));
    // `Dictionary<string, Type>` with the default comparer: ordinal and case-sensitive.
    assert_eq!(legacy("gun"), None);
    assert_eq!(legacy("Gun "), None);
    assert_eq!(legacy("LandscapeMaterial"), None);
    assert_eq!(legacy_keys().count(), 70);
}

#[test]
fn categories() {
    let category = |key: &str| legacy_asset_type(key).unwrap().category();
    assert_eq!(category("Gun"), EAssetType::Item);
    assert_eq!(category("Effect"), EAssetType::Effect);
    assert_eq!(category("Large"), EAssetType::Object);
    assert_eq!(category("NPC"), EAssetType::Object);
    assert_eq!(category("Resource"), EAssetType::Resource);
    assert_eq!(category("Vehicle"), EAssetType::Vehicle);
    assert_eq!(category("Animal"), EAssetType::Animal);
    assert_eq!(category("Mythic"), EAssetType::Mythic);
    assert_eq!(category("Skin"), EAssetType::Skin);
    assert_eq!(category("Spawn"), EAssetType::Spawn);
    assert_eq!(category("Dialogue"), EAssetType::Npc);
    assert_eq!(category("Tag"), EAssetType::None);
    assert_eq!(AssetType::by_name("LandscapeMaterialAsset").unwrap().category(), EAssetType::None);
}

/// `Type.GetType(name, false, true)` called from `UnturnedDat.dll`: Assembly-CSharp classes resolve only when the
/// name says the assembly; the type and assembly names ignore case.
#[test]
fn metadata_type_names() {
    let resolve = |name: &str| resolve_type_name(name).map(|t| t.full_name());
    assert_eq!(
        resolve("SDG.Unturned.LandscapeMaterialAsset, Assembly-CSharp").as_deref(),
        Some("SDG.Unturned.LandscapeMaterialAsset")
    );
    assert_eq!(
        resolve("sdg.unturned.landscapematerialasset, assembly-csharp").as_deref(),
        Some("SDG.Unturned.LandscapeMaterialAsset")
    );
    assert_eq!(
        resolve("SDG.Framework.Foliage.FoliageInfoCollectionAsset, Assembly-CSharp").as_deref(),
        Some("SDG.Framework.Foliage.FoliageInfoCollectionAsset")
    );
    assert_eq!(
        resolve("SDG.Unturned.ItemGunAsset, Assembly-CSharp, Version=0.0.0.0, Culture=neutral, PublicKeyToken=null")
            .as_deref(),
        Some("SDG.Unturned.ItemGunAsset")
    );
    assert_eq!(resolve("SDG.Unturned.LandscapeMaterialAsset"), None);
    assert_eq!(resolve("SDG.Unturned.LandscapeMaterialAsset, OtherAssembly"), None);
    assert_eq!(resolve("LandscapeMaterialAsset, Assembly-CSharp"), None);
}

#[test]
fn friendly_type_names() {
    let friendly = |name: &str| AssetType::by_name(name).unwrap().friendly_type_name();
    assert_eq!(friendly("ItemGunAsset"), "Gun Item");
    assert_eq!(friendly("ItemVehicleRepairToolAsset"), "Vehicle Repair Tool Item");
    assert_eq!(friendly("LandscapeMaterialAsset"), "Landscape Material");
    assert_eq!(friendly("ObjectNPCAsset"), "Object NPC");
    assert_eq!(friendly("NPCRewardsAsset"), "NPC Rewards List");
    assert_eq!(friendly("FoliageInfoCollectionAsset"), "Foliage Info Collection");
    assert_eq!(friendly("SpawnAsset"), "Spawn");
    // Not an `ItemAsset`, so the base rule applies.
    assert_eq!(friendly("ItemCurrencyAsset"), "Item Currency");
}

#[test]
fn every_class_is_listed_once_and_abstract_ones_are_marked() {
    let names: Vec<&str> = AssetType::all().map(AssetType::name).collect();
    assert_eq!(names.len(), 95);
    let mut sorted = names.clone();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len());
    assert!(AssetType::by_name("FoliageInfoAsset").unwrap().is_abstract());
    assert!(!AssetType::by_name("ItemGunAsset").unwrap().is_abstract());
}

#[test]
fn the_resolver_works_through_parse_type() {
    let mut parser = DatParser::new();
    let tree = parser.parse_str("Metadata\n{\n\tType \"SDG.Unturned.ItemGunAsset, Assembly-CSharp\"\n}");
    let metadata = tree.root().try_get_dictionary("Metadata").unwrap();
    let handle = metadata.parse_type("Type", &AssetTypeResolver, None).unwrap().unwrap();
    assert_eq!(AssetType::from_handle(handle).unwrap().name(), "ItemGunAsset");
}
