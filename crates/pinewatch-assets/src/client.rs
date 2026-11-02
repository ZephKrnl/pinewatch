//! Loading the client's assets at startup: every client search location, searched, read and loaded in order.

use pinewatch_io::{InstallRoot, LogSink};
use pinewatch_mono::MonoException;

use crate::load::AssetLoader;
use crate::origin::Origins;
use crate::reader::read_location;
use crate::search::client_search_locations;

/// The client's search locations (`client_search_locations`), each searched and read, and its results taken by the
/// loader before the next location starts (L5). `language` is `Provider.language`. An `Err` is an exception the
/// original doesn't catch on this path.
pub fn load_client_assets(
    install: &InstallRoot,
    language: &str,
    log: &mut dyn LogSink,
) -> Result<AssetLoader, MonoException> {
    let mut origins = Origins::new();
    let locations = client_search_locations(install, &mut origins, log)?;
    let mut loader = AssetLoader::new(origins);
    for location in &locations {
        let results = read_location(install, location, language)?;
        loader.take_results(results, log);
    }
    Ok(loader)
}
