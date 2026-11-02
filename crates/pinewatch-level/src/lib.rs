//! Readers for the original game's map files.
//!
//! Ports from U3-SDK v3.26.3.12. Behaviors not yet confirmed by the Unity harness are listed in
//! `docs/VERIFICATION.md`.

mod hierarchy;
mod landscape;

pub use hierarchy::{HierarchyItem, read_hierarchy_items};
pub use landscape::{
    HEIGHTMAP_RESOLUTION, HOLES_RESOLUTION, LANDSCAPE_TYPE, LandscapeCoord, LandscapeTile, SPLATMAP_LAYERS,
    SPLATMAP_RESOLUTION, TILE_HEIGHT, TILE_SIZE, TileAssets, TileHashes, default_material, read_landscape,
};
