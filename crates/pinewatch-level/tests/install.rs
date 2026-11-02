use pinewatch_assets::{ClientAssetIntegrity, load_client_assets};
use pinewatch_io::{LogLevel, MemoryLog, find_install};
use pinewatch_level::{TileAssets, read_landscape};
use sha1::{Digest, Sha1};

/// Every official map's landscape loads after the assets, as in the game: 160 tiles in total, every heightmap and
/// splatmap read in full (the hash of the bytes consumed is the hash of the whole file), no warnings (so every
/// tile material is among the loaded assets).
#[test]
fn every_installed_landscape_loads() {
    let Ok(install) = find_install(None) else {
        eprintln!("skipped: no Unturned install found");
        return;
    };
    let assets = load_client_assets(&install, "English", &mut MemoryLog::default()).expect("the client's assets");
    let mut total = 0;
    for name in install.read_dir("Maps").unwrap() {
        let name = name.to_string_lossy().into_owned();
        let mut log = MemoryLog::default();
        // Each map starts with the integrity state `ClientAssetIntegrity.Clear` leaves when joining.
        let mut integrity = ClientAssetIntegrity::default();
        let mut context = TileAssets::new(&assets, &mut integrity);
        let tiles = read_landscape(&install, &name, &mut context, &mut log).unwrap_or_else(|e| panic!("{name}: {e}"));
        let Some(tiles) = tiles else { continue };
        for tile in &tiles {
            let (x, y) = (tile.coord.x, tile.coord.y);
            for (kind, folder, extension, hash) in [
                ("heightmap", "Heightmaps", "heightmap", tile.hashes.heightmap),
                ("splatmap", "Splatmaps", "splatmap", tile.hashes.splatmap),
            ] {
                let file = install.read(&format!("Maps/{name}/Landscape/{folder}/Tile_{x}_{y}_Source.{extension}")).unwrap();
                let whole: [u8; 20] = Sha1::digest(&file).into();
                assert_eq!(hash, Some(whole), "{name} {}: {kind} not read in full", tile.coord);
            }
        }
        let warnings: Vec<_> = log.entries.iter().filter(|(level, _)| *level != LogLevel::Info).collect();
        assert!(warnings.is_empty(), "{name}: {warnings:?}");
        total += tiles.len();
    }
    assert_eq!(total, 160);
}
