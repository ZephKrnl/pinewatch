//! The whole install through M2c: every client search location searched and read, every definition loaded,
//! populated as far as ported, and indexed. Asserts the core master bundle, plenty of assets in the index, the item
//! with legacy ID 1 (the Eaglefire) with a friendly name, the landscape materials and the tiles' default one, and
//! prints the messages. The install is only read. Skipped without an install.

use pinewatch_assets::{AssetLoader, AssetReference, Origins, client_search_locations, read_location};
use pinewatch_io::{LogLevel, MemoryLog, find_install};
use pinewatch_mono::{Guid, NetString};

/// `LandscapeTile.DEFAULT_MATERIAL` ("Fallback_Green"), which every tile has in its first slot by default.
const FALLBACK_GREEN: &str = "498ca625072d443a876b2a4f11896018";

#[test]
fn every_search_location_reads_and_loads() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let mut origins = Origins::new();
    let mut log = MemoryLog::default();
    let locations = client_search_locations(&install, &mut origins, &mut log).expect("search locations");
    assert!(!locations.is_empty());
    assert_eq!(locations[0].dir.rel, "Bundles");

    let mut loader = AssetLoader::new(origins);
    for location in &locations {
        let results = read_location(&install, location, "English").expect("the searcher never fails on real names");
        loader.take_results(results, &mut log);
    }

    let core = loader.core_master_bundle().expect("the core master bundle");
    assert_eq!(loader.master_bundles[core].asset_bundle_name, "core.masterbundle");
    assert_eq!(loader.pending(), 0);
    assert!(loader.headers.len() >= 5000, "only {} asset headers", loader.headers.len());

    let mapping = loader.mapping();
    assert!(mapping.asset_list().len() >= 5000, "only {} assets in the index", mapping.asset_list().len());
    let eaglefire = mapping.find_legacy(pinewatch_assets::EAssetType::Item, 1).expect("no item with legacy ID 1");
    assert!(!loader.headers[eaglefire].friendly_name.is_empty(), "item 1 has no friendly name");

    let materials = loader.headers.iter().filter(|header| header.landscape_material.is_some()).count();
    assert!(materials > 0, "no landscape materials");
    let fallback = AssetReference { guid: Guid::try_parse(Some(&NetString::from(FALLBACK_GREEN))).unwrap() };
    let mut find_log = MemoryLog::default();
    assert!(loader.find_landscape_material(fallback, &mut find_log).is_some(), "no Fallback_Green material");

    let warnings = log.entries.iter().filter(|(level, _)| *level == LogLevel::Warn).count();
    let errors = log.entries.iter().filter(|(level, _)| *level == LogLevel::Error).count();
    eprintln!(
        "{} locations, {} master bundles, {} assets ({} indexed, {materials} landscape materials), item 1 is {:?}, \
         {warnings} warnings, {errors} errors, {} reported asset errors",
        locations.len(),
        loader.master_bundles.len(),
        loader.headers.len(),
        mapping.asset_list().len(),
        loader.headers[eaglefire].friendly_name,
        loader.errors.len()
    );
    for (level, text) in log.entries.iter().filter(|(level, _)| *level != LogLevel::Info).take(20) {
        eprintln!("  [{level}] {text}");
    }
}
