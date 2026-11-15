//! Durable authorization identities, never accepted from an RPC parameter.
//!
//! A delivering device alone conflates distinct human origins through one home.
//! Node admission derives this full identity from its authenticated principal;
//! scopes are rechecked on each request and are not part of plan ownership.
use crate::{DeviceId, NodeId};
use serde::{Deserialize, Serialize};

/// A human's authority at the call's authenticated origin; agents have no variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckpointHumanOrigin {
    /// That node's owner token or local socket.
    Owner,
    /// A device paired to the origin node.
    Device {
        /// The origin's device identity, not its home delivery credential.
        device: DeviceId,
    },
}

/// The complete human principal to which a frozen confirmation is pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckpointOwner {
    /// This node's own authenticated token/socket client.
    LocalOwner,
    /// Direct pairing to this node.
    Device {
        /// The authenticated device.
        device: DeviceId,
    },
    /// A human call delivered by the fleet; every component is identity-bearing.
    DelegatedHuman {
        /// The credential authenticated by this destination.
        delivering: DeviceId,
        /// The node where the human initiated the call.
        origin_node: NodeId,
        /// Which human principal on that node.
        origin: CheckpointHumanOrigin,
    },
}
