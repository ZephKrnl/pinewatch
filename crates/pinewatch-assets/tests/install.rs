//! The whole install through what M2c has so far: every client search location searched and read, and every
//! definition loaded up to its header. Asserts the core master bundle and plenty of headers, and prints the
//! messages. The install is only read. Skipped without an install.

use pinewatch_assets::{AssetLoader, Origins, client_search_locations, read_location};
use pinewatch_io::{LogLevel, MemoryLog, find_install};

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

    let warnings = log.entries.iter().filter(|(level, _)| *level == LogLevel::Warn).count();
    let errors = log.entries.iter().filter(|(level, _)| *level == LogLevel::Error).count();
    eprintln!(
        "{} locations, {} master bundles, {} headers, {warnings} warnings, {errors} errors",
        locations.len(),
        loader.master_bundles.len(),
        loader.headers.len()
    );
    for (level, text) in log.entries.iter().filter(|(level, _)| *level != LogLevel::Info).take(20) {
        eprintln!("  [{level}] {text}");
    }
}
