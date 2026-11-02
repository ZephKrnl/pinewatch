//! The game's asset system (`Assets` and `AssetsWorker` in the U3 SDK): search locations, the folder walk,
//! definitions and translations, master bundles, and the asset index. See `docs/superpowers/plans/` (M2c).

mod holiday;
mod landscape_material;
mod load;
mod local;
mod mapping;
mod master_bundle;
mod origin;
mod paths;
mod populate;
mod reader;
mod references;
mod search;
mod types;
mod worker;

pub use holiday::ENPCHoliday;
pub use landscape_material::{EPhysicsMaterial, ERayMask, LandscapeMaterialAsset};
pub use load::{AssetHeader, AssetLoader};
pub use local::Local;
pub use mapping::AssetMapping;
pub use master_bundle::{MasterBundleConfig, asset_bundle_version};
pub use origin::{AssetOrigin, OriginId, Origins};
pub use paths::MonoPath;
pub use reader::{AssetDefinition, ResultItem, read_items, read_location};
pub use references::{AssetReference, ContentReference, parse_guid_or_legacy_id, read_asset_reference};
pub use search::{LevelInfo, SearchLocation, client_search_locations, scan_known_levels};
pub use types::{
    AssetCategory, AssetType, AssetTypeResolver, EAssetType, legacy_asset_type, legacy_keys, resolve_type_name,
};
pub use worker::{SearchEvent, WorkItem, search_location};
