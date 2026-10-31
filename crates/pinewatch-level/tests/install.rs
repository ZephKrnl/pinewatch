use pinewatch_io::{LogLevel, MemoryLog, find_install};
use pinewatch_level::read_landscape;

/// Every official map's landscape loads: 160 tiles in total, every heightmap and splatmap read, no warnings.
#[test]
fn every_installed_landscape_loads() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let mut total = 0;
    for name in install.read_dir("Maps").unwrap() {
        let name = name.to_string_lossy().into_owned();
        let mut log = MemoryLog::default();
        let tiles = read_landscape(&install, &name, &mut log).unwrap_or_else(|e| panic!("{name}: {e}"));
        let Some(tiles) = tiles else { continue };
        for tile in &tiles {
            assert!(tile.hashes.heightmap.is_some(), "{name} {}: heightmap missing", tile.coord);
            assert!(tile.hashes.splatmap.is_some(), "{name} {}: splatmap missing", tile.coord);
        }
        let warnings: Vec<_> = log.entries.iter().filter(|(level, _)| *level != LogLevel::Info).collect();
        assert!(warnings.is_empty(), "{name}: {warnings:?}");
        total += tiles.len();
    }
    assert_eq!(total, 160);
}
