//! Storage-only ownership is derived by the node, never accepted from RPC input.

use crate::{StoreError, codec};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use vessel_proto::creation::CreationError;
use vessel_proto::{NodeId, SessionId};

/// Fixed UUID metadata is covered by each existing retained-byte reservation.
pub(super) const MAX_OWNER_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Owner {
    Person,
    Agent { node: NodeId, parent: SessionId },
}
impl Owner {
    pub(super) fn hash(self, hash: &mut Sha256) {
        match self {
            // Keep the exact historical person digest, including its prefix.
            Self::Person => {}
            Self::Agent { node, parent } => {
                hash.update(b"vessel-agent-creation.v1\0");
                hash.update(node.to_string().as_bytes());
                hash.update([0]);
                hash.update(parent.to_string().as_bytes());
                hash.update([0]);
            }
        }
    }
    pub(super) fn encoded(self) -> Result<String, StoreError> {
        let raw = codec::encode(&self)?;
        if raw.len() > MAX_OWNER_BYTES {
            return Err(CreationError::NamespaceCorrupt.into());
        }
        Ok(raw)
    }
    pub(super) fn decode(raw: &str) -> Result<Self, StoreError> {
        if raw.len() > MAX_OWNER_BYTES {
            return Err(CreationError::NamespaceCorrupt.into());
        }
        serde_json::from_str(raw).map_err(StoreError::CorruptCreationOwner)
    }
}
