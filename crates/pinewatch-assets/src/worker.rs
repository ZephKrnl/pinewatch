//! `AssetsWorker` (`Bundles/AssetsWorker.cs`): the searcher that walks a search location and the reader that reads
//! what it finds.
//!
//! The original runs both on thread-pool threads, for every location at once (L5). Pinewatch walks one location
//! completely, in the searcher's order, and keeps each searcher exception where the searcher raised it.

use pinewatch_io::InstallRoot;
use pinewatch_io::mono_path;
use pinewatch_mono::MonoException;

use crate::paths::{self, MonoPath, STYLE};

/// What the searcher queues for the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkItem {
    /// A `MasterBundle.dat` found in a folder.
    MasterBundle(MonoPath),
    /// An asset definition file.
    Asset(MonoPath),
}

/// One step of the searcher: a work item, or an exception it caught and reports with this message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchEvent {
    Item(WorkItem),
    Exception { message: String, exception: MonoException },
}

/// `SearcherThreadMain` for one location: breadth first, each folder's `MasterBundle.dat`, then `FindAssets`, then
/// its subfolders queued. An `Err` is an exception the searcher doesn't catch (from `Path.Combine` or
/// `Path.GetFileName` on an impossible name): in the original it ends the searcher thread and the location never
/// finishes reading.
pub fn search_location(install: &InstallRoot, root: &MonoPath) -> Result<Vec<SearchEvent>, MonoException> {
    let mut events = Vec::new();
    let mut search_paths = std::collections::VecDeque::from([root.clone()]);
    while let Some(dir) = search_paths.pop_front() {
        let master_bundle_config = dir.combine("MasterBundle.dat")?;
        // `File.Exists` returns false instead of throwing, so its catch never runs.
        if install.exists(&master_bundle_config.rel) {
            events.push(SearchEvent::Item(WorkItem::MasterBundle(master_bundle_config)));
        }

        find_assets(install, &dir, &mut events)?;

        match paths::directories(install, &dir) {
            Ok(subdirectories) => search_paths.extend(subdirectories),
            Err(exception) => events.push(SearchEvent::Exception {
                message: format!("Caught exception finding asset subdirectories in: \"{}\"", dir.full),
                exception,
            }),
        }
    }
    Ok(events)
}

/// `FindAssets`: `<Folder>.asset`, else `<Folder>.dat`, else `Asset.dat`, else every `*.asset`.
fn find_assets(install: &InstallRoot, dir: &MonoPath, events: &mut Vec<SearchEvent>) -> Result<(), MonoException> {
    let folder_name = mono_path::get_file_name(STYLE, &dir.full)?;
    for name in [format!("{folder_name}.asset"), format!("{folder_name}.dat"), "Asset.dat".to_string()] {
        let test_path = dir.combine(&name)?;
        if install.exists(&test_path.rel) {
            events.push(SearchEvent::Item(WorkItem::Asset(test_path)));
            return Ok(());
        }
    }
    match paths::asset_files(install, dir) {
        Ok(files) => events.extend(files.into_iter().map(|file| SearchEvent::Item(WorkItem::Asset(file)))),
        Err(exception) => events.push(SearchEvent::Exception {
            message: format!("Caught exception checking for assets in: \"{}\"", dir.full),
            exception,
        }),
    }
    Ok(())
}
