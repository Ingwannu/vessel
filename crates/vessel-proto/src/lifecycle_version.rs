//! Availability of the lifecycle metadata on negotiated connections.
//! Storage is independent: no peer may see an unassigned field.

use crate::ProtocolVersion;

/// The lifecycle metadata's assigned protocol, pending integration.
///
/// Assignment requires the shared required-protocol gate, CLI run/resume,
/// list/update, MCP create/list/update, and the native/web metadata views
/// to validate and handle labels. The integrator sets Some(version) only
/// when these consumers and background discovery/inheritance are ready;
/// None hides labels and listing from every peer.
pub const SINCE: Option<ProtocolVersion> = None;

/// Share the reply projection rule with export producers, so an unreadable
/// transfer is refused before it could discard metadata at a peer.
#[must_use]
pub fn visible(assigned: Option<ProtocolVersion>, peer: ProtocolVersion) -> bool {
    assigned.is_some_and(|since| peer >= since)
}
