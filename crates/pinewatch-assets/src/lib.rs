//! The game's asset system (`Assets` and `AssetsWorker` in the U3 SDK): search locations, the folder walk,
//! definitions and translations, master bundles, and the asset index. See `docs/superpowers/plans/` (M2c).

mod origin;
mod paths;
mod search;
mod worker;

pub use origin::{AssetOrigin, OriginId, Origins};
pub use paths::MonoPath;
pub use search::{LevelInfo, SearchLocation, client_search_locations, scan_known_levels};
pub use worker::{SearchEvent, WorkItem, search_location};
