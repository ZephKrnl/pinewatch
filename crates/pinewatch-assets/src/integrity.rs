//! `ClientAssetIntegrity` (`Bundles/ClientAssetIntegrity.cs`): the asset GUIDs a client checks with the server, and
//! the ones the server knows are missing. Sending the requests comes with the network code; the state and the
//! messages are here. With `LOG_CLIENT_ASSET_INTEGRITY` off in the shipped build, only the two messages below exist.

use std::collections::HashSet;

use pinewatch_io::{LogLevel, LogSink};
use pinewatch_mono::Guid;

/// The integrity state. Only whether a GUID is in each set is ever read, so the sets' order doesn't matter;
/// the pending requests keep theirs.
#[derive(Debug, Default)]
pub struct ClientAssetIntegrity {
    validated_guids: HashSet<Guid>,
    server_known_missing_guids: HashSet<Guid>,
    pending_validation: Vec<(Guid, Option<usize>)>,
}

impl ClientAssetIntegrity {
    /// `Clear`, before joining a server: every set emptied (the send timer comes with the network code).
    pub fn clear(&mut self) {
        self.validated_guids.clear();
        self.server_known_missing_guids.clear();
        self.pending_validation.clear();
    }

    /// `ServerAddKnownMissingAsset(guid, context)`: records an asset the server knows is missing, so clients
    /// missing it too aren't kicked. Logged the first time.
    pub fn server_add_known_missing_asset(&mut self, guid: Guid, context: &str, log: &mut dyn LogSink) {
        if guid != Guid::EMPTY && self.server_known_missing_guids.insert(guid) {
            let guid = guid.to_string_n();
            log.log(
                LogLevel::Info,
                &format!("Context \"{context}\" known missing asset {guid}, server will not kick clients for this"),
            );
        }
    }

    /// Whether `server_add_known_missing_asset` recorded `guid`.
    pub fn is_server_known_missing(&self, guid: Guid) -> bool {
        self.server_known_missing_guids.contains(&guid)
    }

    /// `QueueRequest(guid, asset, context)`: queues a GUID for the server to check, once. `asset` is the asset it
    /// found (an index into the loader's headers), and a missing one is warned about when queued.
    pub fn queue_request(&mut self, guid: Guid, asset: Option<usize>, context: &str, log: &mut dyn LogSink) {
        if guid == Guid::EMPTY || !self.validated_guids.insert(guid) {
            return;
        }
        if asset.is_none() {
            log.log(LogLevel::Warn, &format!("Context \"{context}\" missing asset {}", guid.to_string_n()));
        }
        self.pending_validation.push((guid, asset));
    }

    /// `pendingValidation`: the queued GUIDs with the asset each one found, in order.
    pub fn pending_validation(&self) -> &[(Guid, Option<usize>)] {
        &self.pending_validation
    }
}
