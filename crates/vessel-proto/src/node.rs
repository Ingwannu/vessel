//! Node identity and protocol versions.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::key::NodePublicKey;

/// A wire protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
    /// Wrap a raw version number.
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }

    /// The raw number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The protocol this build speaks. Bump on any breaking change to a wire type
/// and record the change in `docs/PROTOCOL.md`.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion(24);

/// The protocol version that speaks generated names (ADR 0062). The
/// integrator assigns this after the CLI, web and native clients can read
/// origins and configure generated names; `None` means no peer speaks it.
/// Callers compare against it; this track does not copy it into
/// [`PROTOCOL_VERSION`].
pub const GENERATED_NAMES: Option<ProtocolVersion> = None;

/// Whether this build puts generated-name fields on the wire. False until
/// [`GENERATED_NAMES`] is at most [`PROTOCOL_VERSION`].
#[must_use]
pub const fn speaks_generated_names() -> bool {
    matches!(GENERATED_NAMES, Some(version) if version.get() <= PROTOCOL_VERSION.get())
}

/// The oldest protocol this build still accepts. Versions 3 to 24 only add
/// methods, events (with stand-ins for older peers) and defaulted fields,
/// so version 2 peers keep working.
pub const MIN_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion(2);

/// An inclusive range of protocol versions a peer can speak.
///
/// [Decision Log]
/// - Purpose: fleet nodes, the CLI and future editor plugins will run
///   different versions at the same time.
/// - Alternatives: exact-match versions (Zed's collab server answers 426 on
///   any mismatch, workable only because Zed ships client and server
///   together); no envelope version at all (Paseo, which now carries hundreds
///   of per-field compatibility shims).
/// - Choice: both sides state a range in the handshake and use the highest
///   common version, or refuse with `ErrorCode::ProtocolMismatch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProtocolRange {
    /// Oldest supported version.
    pub min: ProtocolVersion,
    /// Newest supported version.
    pub max: ProtocolVersion,
}

impl ProtocolRange {
    /// The range this build supports.
    pub const CURRENT: Self = Self {
        min: MIN_PROTOCOL_VERSION,
        max: PROTOCOL_VERSION,
    };

    /// The highest version both ranges contain, if any.
    #[must_use]
    pub fn negotiate(self, other: Self) -> Option<ProtocolVersion> {
        let high = self.max.min(other.max);
        let low = self.min.max(other.min);
        (low <= high).then_some(high)
    }
}

/// Whether a node is taking new work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    /// Accepting new sessions.
    #[default]
    Serving,
    /// Asked to stop once idle (`node.stop` with `when_idle`): live sessions
    /// continue, new ones are refused with `ErrorCode::Unavailable`.
    Draining,
}

/// Facts about a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NodeInfo {
    /// Stable id for this node.
    pub id: NodeId,
    /// Human-readable name (defaults to the hostname).
    pub name: String,
    /// OS hostname.
    pub hostname: String,
    /// `std::env::consts::OS`.
    pub os: String,
    /// `std::env::consts::ARCH`.
    pub arch: String,
    /// Version of the node binary.
    pub version: String,
    /// Protocols the node speaks.
    pub protocol: ProtocolRange,
    /// When the node process started.
    pub started_at: DateTime<Utc>,
    /// Whether it takes new sessions (protocol 3; absent means serving).
    #[serde(default)]
    pub state: NodeState,
    /// The node's static key for secure channels (protocol 4; absent from
    /// older nodes and from nodes without a key yet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<NodePublicKey>,
    /// System IANA time zone, detected at start (protocol 13). Absent on
    /// older nodes or when the system cannot supply an IANA name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<crate::automation::TimeZoneName>,
}

impl NodeInfo {
    /// Remove fields unavailable to a negotiated older peer.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        if protocol.get() < 13 && self.timezone.is_some() {
            let mut node = self.clone();
            node.timezone = None;
            Some(node)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn range(min: u32, max: u32) -> ProtocolRange {
        ProtocolRange {
            min: ProtocolVersion::new(min),
            max: ProtocolVersion::new(max),
        }
    }

    #[test]
    fn negotiation_picks_highest_common_version() {
        assert_eq!(
            range(2, 5).negotiate(range(3, 4)),
            Some(ProtocolVersion::new(4))
        );
        assert_eq!(
            range(2, 3).negotiate(range(3, 9)),
            Some(ProtocolVersion::new(3))
        );
        assert_eq!(range(2, 3).negotiate(range(4, 9)), None);
    }
}

#[cfg(test)]
mod timezone_tests {
    use super::*;
    use crate::{Outcome, Reply, RequestId, ServerFrame, Welcome};

    fn node() -> NodeInfo {
        NodeInfo {
            id: NodeId::random(),
            name: "fixture".to_owned(),
            hostname: "fixture".to_owned(),
            os: "test".to_owned(),
            arch: "test".to_owned(),
            version: "0".to_owned(),
            protocol: ProtocolRange::CURRENT,
            started_at: DateTime::UNIX_EPOCH,
            state: NodeState::Serving,
            public_key: None,
            timezone: Some("Asia/Seoul".parse().unwrap()),
        }
    }

    #[test]
    fn older_nodes_default_timezone_and_older_clients_never_receive_it() {
        let old = NodeInfo {
            timezone: None,
            ..node()
        };
        let json = serde_json::to_string(&old).unwrap();
        assert!(!json.contains("timezone"));
        assert_eq!(serde_json::from_str::<NodeInfo>(&json).unwrap(), old);
        let node = node();
        for frame in [
            ServerFrame::Welcome(Welcome {
                protocol: ProtocolVersion::new(12),
                node: node.clone(),
            }),
            ServerFrame::Response {
                id: RequestId::new(1),
                outcome: Outcome::Ok(Reply::NodeInfo(node.clone())),
            },
        ] {
            assert_eq!(frame.for_protocol(PROTOCOL_VERSION), None);
            let old_frame = frame.for_protocol(ProtocolVersion::new(12)).unwrap();
            assert!(
                !serde_json::to_string(&old_frame)
                    .unwrap()
                    .contains("timezone")
            );
        }
    }
}
