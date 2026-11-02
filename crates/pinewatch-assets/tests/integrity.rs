//! `ClientAssetIntegrity`'s known-missing and validation sets (M2c task 11).

use pinewatch_assets::ClientAssetIntegrity;
use pinewatch_io::{LogLevel, MemoryLog};
use pinewatch_mono::{Guid, NetString};

const A: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";
const B: &str = "2b2a3d4c5e6f708192a3b4c5d6e7f809";

fn guid(text: &str) -> Guid {
    Guid::try_parse(Some(&NetString::from(text))).unwrap()
}

/// `ServerAddKnownMissingAsset`: logged the first time a GUID is added, never for `Guid.Empty`.
#[test]
fn known_missing_assets_are_logged_once() {
    let mut integrity = ClientAssetIntegrity::default();
    let mut log = MemoryLog::default();
    integrity.server_add_known_missing_asset(guid(A), "First", &mut log);
    integrity.server_add_known_missing_asset(guid(A), "Second", &mut log);
    integrity.server_add_known_missing_asset(Guid::EMPTY, "Empty", &mut log);
    assert_eq!(log.entries, [(
        LogLevel::Info,
        format!("Context \"First\" known missing asset {A}, server will not kick clients for this")
    )]);
    assert!(integrity.is_server_known_missing(guid(A)));
    assert!(!integrity.is_server_known_missing(guid(B)));

    integrity.clear();
    assert!(!integrity.is_server_known_missing(guid(A)));
    integrity.server_add_known_missing_asset(guid(A), "Again", &mut log);
    assert_eq!(log.entries.len(), 2);
}

/// `QueueRequest(guid, asset, context)`: each GUID is queued once, in order; a missing asset is warned about when it
/// is queued.
#[test]
fn requests_are_queued_once_and_missing_assets_warned_about() {
    let mut integrity = ClientAssetIntegrity::default();
    let mut log = MemoryLog::default();
    integrity.queue_request(guid(A), None, "Tile", &mut log);
    integrity.queue_request(guid(A), None, "Tile again", &mut log);
    integrity.queue_request(guid(B), Some(7), "Found", &mut log);
    integrity.queue_request(Guid::EMPTY, None, "Empty", &mut log);
    assert_eq!(log.entries, [(LogLevel::Warn, format!("Context \"Tile\" missing asset {A}"))]);
    assert_eq!(integrity.pending_validation(), [(guid(A), None), (guid(B), Some(7))]);

    integrity.clear();
    assert!(integrity.pending_validation().is_empty());
    integrity.queue_request(guid(A), None, "After clear", &mut log);
    assert_eq!(log.entries.len(), 2);
}
