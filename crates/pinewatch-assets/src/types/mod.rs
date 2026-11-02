//! The asset classes (`Asset` and everything deriving from it in the U3 SDK), the legacy `Type` keys
//! (`TypeRegistryDictionary` with `UnturnedNexus`' registrations), and `Metadata.Type` resolution.

mod table;

use pinewatch_dat::{TypeHandle, TypeResolver};
use pinewatch_mono::{EnumInfo, EnumUnderlying, MonoException, NetEnum, NetString};

/// `EAssetType`, the category that decides an asset's legacy ID table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EAssetType {
    None,
    Item,
    Effect,
    Object,
    Resource,
    Vehicle,
    Animal,
    Mythic,
    Skin,
    Spawn,
    Npc,
}

impl EAssetType {
    const ALL: [EAssetType; 11] = [
        Self::None,
        Self::Item,
        Self::Effect,
        Self::Object,
        Self::Resource,
        Self::Vehicle,
        Self::Animal,
        Self::Mythic,
        Self::Skin,
        Self::Spawn,
        Self::Npc,
    ];

    /// The enum's value.
    pub fn bits(self) -> i32 {
        Self::ALL.iter().position(|&category| category == self).unwrap_or(0) as i32
    }

    /// The member with this value, if one has it.
    pub fn from_bits(bits: i32) -> Option<EAssetType> {
        usize::try_from(bits).ok().and_then(|index| Self::ALL.get(index).copied())
    }
}

/// An `EAssetType` value as C# holds it, which may be one no member has (`AssetCategory 42` parses). The default
/// is `NONE`, as `default(EAssetType)` is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AssetCategory(pub i32);

impl AssetCategory {
    pub fn known(self) -> Option<EAssetType> {
        EAssetType::from_bits(self.0)
    }
}

impl From<EAssetType> for AssetCategory {
    fn from(category: EAssetType) -> Self {
        Self(category.bits())
    }
}

impl std::fmt::Display for AssetCategory {
    /// `Enum.ToString()`: the member's name, or the number for a value no member has.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.known() {
            Some(category) => f.write_str(Self::INFO.names[category.bits() as usize]),
            None => write!(f, "{}", self.0),
        }
    }
}

impl NetEnum for AssetCategory {
    const INFO: EnumInfo = EnumInfo {
        underlying: EnumUnderlying::I32,
        names: &["NONE", "ITEM", "EFFECT", "OBJECT", "RESOURCE", "VEHICLE", "ANIMAL", "MYTHIC", "SKIN", "SPAWN", "NPC"],
        values: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    };

    fn from_bits(bits: u64) -> Self {
        Self(bits as i32)
    }
}

/// Which `FriendlyName` an asset class has: `Asset`'s (its name) or the override of the nearest class that has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FriendlyName {
    /// `Asset.FriendlyName`: the name.
    Base,
    /// The localized `Name` (`AnimalAsset`, `ObjectAsset`, `ResourceAsset`, `RoadAsset`, `VehicleAsset`).
    Localized,
    /// `ItemAsset`: `FormatOrEmpty("Name")`, or the name when that is empty.
    ItemName,
    /// `LandscapeMaterialAsset`: the name without `_Material`, underscores as spaces.
    LandscapeMaterial,
    /// `TagAsset`: the localized `Name` if the translation has one, else the name.
    Tag,
    /// `VendorAsset`: the localized `Name` through `ItemTool.filterRarityRichText` and `RichTextUtil.replaceColorTags`.
    Vendor,
    /// `ServerListCurationAsset`: the curation file's name, or the name.
    ServerListCuration,
}

/// One class's facts, as the SDK declares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AssetTypeInfo {
    namespace: &'static str,
    name: &'static str,
    /// `assetCategory`, inherited from the nearest class that overrides it.
    category: EAssetType,
    is_abstract: bool,
    /// Derives from `ItemAsset` (or is it), which changes `GetTypeFriendlyName`.
    is_item: bool,
    friendly: FriendlyName,
}

/// A class deriving from `Asset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetType(u16);

impl AssetType {
    fn info(self) -> &'static AssetTypeInfo {
        &table::ASSET_TYPES[usize::from(self.0)]
    }

    /// Every asset class, sorted by name.
    pub fn all() -> impl Iterator<Item = AssetType> {
        (0..table::ASSET_TYPES.len() as u16).map(AssetType)
    }

    /// The class with exactly this name (without namespace).
    pub fn by_name(name: &str) -> Option<AssetType> {
        Self::all().find(|t| t.name() == name)
    }

    /// The class name, for example `ItemGunAsset`.
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// `Type.FullName` (and `ToString()`), for example `SDG.Unturned.ItemGunAsset`.
    pub fn full_name(self) -> String {
        format!("{}.{}", self.info().namespace, self.info().name)
    }

    /// `assetCategory`. `RedirectorAsset` returns the category its data names; this is its default, `NONE`.
    pub fn category(self) -> EAssetType {
        self.info().category
    }

    /// Derives from `ItemAsset` (or is it).
    pub fn is_item(self) -> bool {
        self.info().is_item
    }

    pub(crate) fn friendly_name_rule(self) -> FriendlyName {
        self.info().friendly
    }

    /// Abstract classes can't be created (`Activator.CreateInstance` throws).
    pub fn is_abstract(self) -> bool {
        self.info().is_abstract
    }

    /// `GetTypeNameWithoutSuffix`: the class name without a final `Asset`.
    pub fn name_without_suffix(self) -> &'static str {
        let name = self.name();
        name.strip_suffix("Asset").unwrap_or(name)
    }

    /// `GetTypeFriendlyName`: the name without `Asset`, with a space before each upper-case letter that follows a
    /// letter that isn't upper case. `ItemAsset` moves a leading `Item` to the end (`Gun Item`), and
    /// `NPCRewardsAsset` is `NPC Rewards List`.
    pub fn friendly_type_name(self) -> String {
        if self.name() == "NPCRewardsAsset" {
            return "NPC Rewards List".to_string();
        }
        // `char.IsUpper` on class names, which are ASCII.
        let mut friendly = String::new();
        let mut previous_upper = false;
        for (index, letter) in self.name_without_suffix().chars().enumerate() {
            let upper = letter.is_ascii_uppercase();
            if index > 0 && upper && !previous_upper {
                friendly.push(' ');
            }
            friendly.push(letter);
            previous_upper = upper;
        }
        if self.info().is_item
            && let Some(rest) = friendly.strip_prefix("Item ")
        {
            return format!("{rest} Item");
        }
        friendly
    }

    /// The handle `pinewatch-dat`'s `parse_type` returns for this class.
    pub fn handle(self) -> TypeHandle {
        TypeHandle(u32::from(self.0))
    }

    pub fn from_handle(handle: TypeHandle) -> Option<AssetType> {
        let index = u16::try_from(handle.0).ok()?;
        (usize::from(index) < table::ASSET_TYPES.len()).then_some(AssetType(index))
    }
}

/// `Assets.assetTypes.getType(key)`: the class registered for a legacy `Type` key, compared ordinally.
pub fn legacy_asset_type(key: &str) -> Option<AssetType> {
    let index = table::LEGACY_KEYS.binary_search_by(|(candidate, _)| candidate.cmp(&key)).ok()?;
    Some(AssetType(table::LEGACY_KEYS[index].1))
}

/// Every legacy key and its class, sorted by key.
pub fn legacy_keys() -> impl Iterator<Item = (&'static str, AssetType)> {
    table::LEGACY_KEYS.iter().map(|&(key, index)| (key, AssetType(index)))
}

/// `Type.GetType(name, throwOnError: false, ignoreCase: true)` as `DatValueEx.ParseType` calls it, from
/// `UnturnedDat.dll`, for the asset classes. Without an assembly name the search covers only `UnturnedDat.dll` and
/// `mscorlib.dll`, which hold no asset class. With one, `Assembly-CSharp` (any case, any further properties such as
/// `Version=`) and the full class name in any case find the class. Other types `Type.GetType` would find (a
/// non-asset class, an array type) aren't known here (V19).
pub fn resolve_type_name(name: &str) -> Option<AssetType> {
    let (type_name, assembly) = name.split_once(',')?;
    let simple_name = assembly.split(',').next().unwrap_or("").trim();
    if !simple_name.eq_ignore_ascii_case("Assembly-CSharp") {
        return None;
    }
    AssetType::all().find(|t| t.full_name().eq_ignore_ascii_case(type_name))
}

/// The `TypeResolver` for asset definitions: [`resolve_type_name`], as `pinewatch-dat`'s `parse_type` handles.
#[derive(Debug, Clone, Copy, Default)]
pub struct AssetTypeResolver;

impl TypeResolver for AssetTypeResolver {
    fn resolve(&self, assembly_qualified_name: &NetString) -> Result<Option<TypeHandle>, MonoException> {
        Ok(resolve_type_name(&assembly_qualified_name.to_string_lossy()).map(AssetType::handle))
    }
}
