//! `Assets.AssetMapping` (`Bundles/Assets.cs`): the legacy ID tables per category, the GUID dictionary and the asset
//! list. Entries are indexes into [`crate::AssetLoader::headers`].

use pinewatch_mono::{ExceptionKind, Guid, MonoDictionary, MonoException, sr};

use crate::types::{AssetCategory, EAssetType};

/// One asset mapping, as `new AssetMapping()` builds it.
#[derive(Debug, Clone)]
pub struct AssetMapping {
    /// `legacyAssetsTable`, in the constructor's order.
    legacy_assets_table: Vec<(EAssetType, MonoDictionary<u16, usize>)>,
    /// `assetDictionary`.
    pub(crate) asset_dictionary: MonoDictionary<Guid, usize>,
    /// `assetList`.
    pub(crate) asset_list: Vec<usize>,
    /// `modificationCounter`.
    pub(crate) modification_counter: i32,
}

impl Default for AssetMapping {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetMapping {
    pub fn new() -> Self {
        let categories = [
            EAssetType::Item,
            EAssetType::Effect,
            EAssetType::Object,
            EAssetType::Resource,
            EAssetType::Vehicle,
            EAssetType::Animal,
            EAssetType::Mythic,
            EAssetType::Skin,
            EAssetType::Spawn,
            EAssetType::Npc,
        ];
        Self {
            legacy_assets_table: categories.into_iter().map(|category| (category, MonoDictionary::new())).collect(),
            asset_dictionary: MonoDictionary::new(),
            asset_list: Vec::new(),
            modification_counter: 0,
        }
    }

    /// `legacyAssetsTable[category]`: the dictionary's indexer throws `KeyNotFoundException` for a category it
    /// doesn't hold (`NONE`, or a value no member has).
    pub(crate) fn table_mut(
        &mut self,
        category: AssetCategory,
    ) -> Result<&mut MonoDictionary<u16, usize>, MonoException> {
        self.legacy_assets_table
            .iter_mut()
            .find(|(candidate, _)| AssetCategory::from(*candidate) == category)
            .map(|(_, table)| table)
            .ok_or_else(|| {
                let message = sr::ARG_KEY_NOT_FOUND_WITH_KEY.replace("{0}", &category.to_string());
                MonoException::new(ExceptionKind::KeyNotFound, message)
            })
    }

    /// `assetDictionary.TryGetValue(guid)`.
    pub fn find_by_guid(&self, guid: &Guid) -> Option<usize> {
        self.asset_dictionary.try_get_value(guid).copied()
    }

    /// `legacyAssetsTable[category].TryGetValue(id)`.
    pub fn find_legacy(&self, category: EAssetType, id: u16) -> Option<usize> {
        let (_, table) = self.legacy_assets_table.iter().find(|(candidate, _)| *candidate == category)?;
        table.try_get_value(&id).copied()
    }

    /// The entries of one legacy table, in its enumeration order.
    pub fn legacy_entries(&self, category: EAssetType) -> Vec<(u16, usize)> {
        self.legacy_assets_table
            .iter()
            .find(|(candidate, _)| *candidate == category)
            .map(|(_, table)| table.iter().map(|(&id, &index)| (id, index)).collect())
            .unwrap_or_default()
    }

    /// `assetList`, in order.
    pub fn asset_list(&self) -> &[usize] {
        &self.asset_list
    }

    /// `modificationCounter`.
    pub fn modification_counter(&self) -> i32 {
        self.modification_counter
    }
}
