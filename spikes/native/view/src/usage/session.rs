//! A conversation quota read belongs to its actual selected binding epoch.
use crate::devices::MachineKey;
use vessel_proto::{EventSeq, SessionId};

/// Rebind records invalidate a pending answer even if the session id stays the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSelection {
    /// Parsed book key used by the link registry.
    pub machine: MachineKey,
    /// Existing conversation identity.
    pub session: SessionId,
    /// Last observed launch/profile replacement record, when one is known.
    pub binding: Option<EventSeq>,
}
