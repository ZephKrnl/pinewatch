//! The game's asset system (`Assets` and `AssetsWorker` in the U3 SDK): search locations, the folder walk,
//! definitions and translations, master bundles, and the asset index. See `docs/superpowers/plans/` (M2c).

mod caching;
mod client;
mod holiday;
mod integrity;
mod landscape_material;
mod level_info;
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

pub use caching::{
    CachingAssetRef, CachingBcAssetRef, parse_asset_ref, parse_guid_or_legacy_id_v2, try_parse_asset_ref,
    try_parse_bc_asset_ref,
};
pub use client::load_client_assets;
pub use holiday::ENPCHoliday;
pub use integrity::ClientAssetIntegrity;
pub use landscape_material::{EPhysicsMaterial, ERayMask, LandscapeMaterialAsset};
pub use level_info::{ELevelSize, ELevelType, LevelFile, read_level_file, try_parse_packed_version};
pub use load::{AssetHeader, AssetLoader};
pub use local::Local;
pub use mapping::AssetMapping;
pub use master_bundle::{MasterBundleConfig, asset_bundle_version};
pub use origin::{AssetOrigin, OriginId, Origins};
pub use paths::MonoPath;
pub use reader::{AssetDefinition, ResultItem, read_items, read_location};
pub use references::{
    AssetReference, AudioReference, ContentReference, MasterBundleReference, parse_guid_or_legacy_id,
    read_asset_reference, read_audio_reference, read_master_bundle_reference,
};
pub use search::{LevelInfo, SearchLocation, client_search_locations, scan_known_levels};
pub use types::{
    AssetCategory, AssetType, AssetTypeResolver, EAssetType, legacy_asset_type, legacy_keys, resolve_type_name,
};
pub use worker::{SearchEvent, WorkItem, search_location};
