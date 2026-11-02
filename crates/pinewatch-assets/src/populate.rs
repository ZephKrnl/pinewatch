//! The parts of each asset class's `PopulateAsset` ported so far: what the asset index needs, and the classes
//! ported whole. That is the friendly name (the 10 classes that override `FriendlyName` set it from their
//! translation or name), `ItemAsset.isPro`, and all of `RedirectorAsset`'s and `LandscapeMaterialAsset`'s.
//! Everything else the classes read, report or throw comes as each class is ported (V28).

use pinewatch_mono::{Guid, MonoException};

use crate::landscape_material::LandscapeMaterialAsset;
use crate::load::AssetHeader;
use crate::types::{AssetCategory, EAssetType, FriendlyName};

/// The class's own `PopulateAsset`, as far as it is ported. `current_master_bundle` is `Assets.currentMasterBundle`'s
/// bundle name. Returns the messages it passes to `Assets.ReportError`, in order, and then `Err` if it threw.
pub(crate) fn populate(
    header: &mut AssetHeader,
    current_master_bundle: Option<String>,
) -> (Vec<String>, Result<(), MonoException>) {
    let mut reports = Vec::new();
    let mut populated = Ok(());
    let (friendly_name, is_pro, category, target_guid, landscape_material) = {
        let local = header.localization();
        let data = header.data();
        let friendly_name = match header.asset_type.friendly_name_rule() {
            FriendlyName::Base | FriendlyName::ServerListCuration => header.name.clone(),
            FriendlyName::Localized => local.format("Name").to_string_lossy(),
            FriendlyName::ItemName => {
                let item_name = local.format_or_empty("Name");
                if item_name.is_empty() { header.name.clone() } else { item_name.to_string_lossy() }
            }
            FriendlyName::LandscapeMaterial => {
                let name = header.name.strip_suffix("_Material").unwrap_or(&header.name);
                name.replace('_', " ")
            }
            FriendlyName::Tag => {
                if local.has("Name") { local.format("Name").to_string_lossy() } else { header.name.clone() }
            }
            // The rarity and color tag filters change only text inside `<...>` tags (V28).
            FriendlyName::Vendor => local.format("Name").to_string_lossy(),
        };
        // `ItemAsset.PopulateAsset`: `isPro = data.ContainsKey("Pro")`.
        let is_pro = header.asset_type.is_item() && data.contains_key("Pro");
        let mut category = AssetCategory::from(header.asset_type.category());
        let mut target_guid = None;
        let mut landscape_material = None;
        match header.asset_type.name() {
            "RedirectorAsset" => {
                category = data.parse_enum("AssetCategory", AssetCategory::from(EAssetType::None));
                if header.id > 0 && category == AssetCategory::from(EAssetType::None) {
                    reports.push("legacy ID was assigned but AssetCategory was not".to_string());
                }
                let parsed = data.try_parse_guid("TargetAsset");
                if parsed.is_none() {
                    reports.push("unable to parse TargetAsset".to_string());
                }
                target_guid = Some(parsed.unwrap_or(Guid::EMPTY));
            }
            // Nothing can see an asset whose `PopulateAsset` threw, so its partly set fields aren't kept.
            "LandscapeMaterialAsset" => match LandscapeMaterialAsset::populate(&data, current_master_bundle) {
                Ok(material) => landscape_material = Some(material),
                Err(exception) => populated = Err(exception),
            },
            _ => {}
        }
        (friendly_name, is_pro, category, target_guid, landscape_material)
    };
    header.friendly_name = friendly_name;
    header.is_pro = is_pro;
    header.category = category;
    header.target_guid = target_guid;
    header.landscape_material = landscape_material;
    (reports, populated)
}
