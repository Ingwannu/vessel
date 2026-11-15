//! A durable detach changes delegation without rewriting creation provenance.

use crate::{Creator, SessionId, SessionInfo, SessionRef};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stored once per managed child, surviving lost replies, resume and restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetachReceipt {
    former_parent: SessionRef,
    detached_at: DateTime<Utc>,
}

impl DetachReceipt {
    /// The writer supplies the actual former parent and commit time.
    #[must_use]
    pub const fn new(former_parent: SessionRef, detached_at: DateTime<Utc>) -> Self {
        Self {
            former_parent,
            detached_at,
        }
    }
    /// The full address, including a different node's identity.
    #[must_use]
    pub const fn former_parent(&self) -> SessionRef {
        self.former_parent
    }
    /// A repeated request returns this original time, never the retry time.
    #[must_use]
    pub const fn detached_at(&self) -> DateTime<Utc> {
        self.detached_at
    }
}

/// A malformed receipt must not accidentally authorize a different lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the detach receipt does not match this session's creator")]
pub struct InvalidParentage;

/// Present-day delegation, separate from the immutable Creator audit field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parentage {
    /// A person created this session; it never was a managed child.
    Root,
    /// An attached child, possibly on another node.
    Attached(SessionRef),
    /// A former child explicitly made independent, with its original receipt.
    Detached(DetachReceipt),
}

impl Parentage {
    /// Resolve only after validating the receipt against immutable provenance.
    /// # Errors
    /// A receipt names a different creator or belongs to a never-child.
    pub fn of(creator: &Creator, receipt: Option<DetachReceipt>) -> Result<Self, InvalidParentage> {
        let parent = match creator {
            Creator::Agent { node, session } => Some(SessionRef {
                node: *node,
                session: *session,
            }),
            Creator::Owner | Creator::Device { .. } => None,
        };
        match (parent, receipt) {
            (None, None) => Ok(Self::Root),
            (Some(parent), None) => Ok(Self::Attached(parent)),
            (Some(parent), Some(receipt)) if parent == receipt.former_parent => {
                Ok(Self::Detached(receipt))
            }
            (None | Some(_), Some(_)) => Err(InvalidParentage),
        }
    }
    /// Ownership and navigation stop traversing at a detached boundary.
    #[must_use]
    pub const fn parent(self) -> Option<SessionRef> {
        match self {
            Self::Attached(parent) => Some(parent),
            Self::Root | Self::Detached(_) => None,
        }
    }
}

impl SessionInfo {
    /// Current delegation after validating immutable provenance against the receipt.
    /// # Errors
    /// Corrupt or mismatched receipt; callers must refuse rather than infer ownership.
    pub fn parentage(&self) -> Result<Parentage, InvalidParentage> {
        Parentage::of(&self.created_by, self.detach_receipt)
    }
}

/// Planned session.detach input; only the target id comes from the caller.
/// Method registration awaits the central pending-method contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetachSession {
    /// The child on the receiving node; parent and workspace are authoritative.
    pub session: SessionId,
}

/// A retry distinguishes a committed detach from a new mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DetachDisposition {
    /// This request committed the first detach.
    Detached,
    /// The original receipt was already durable; nothing changed.
    AlreadyDetached,
}

/// Planned session.detach result; session carries the unchanged workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetachedSession {
    /// Current authoritative session with identical process/history identity.
    pub session: Box<SessionInfo>,
    /// Original typed parent and commit time, including on a repeat.
    pub receipt: DetachReceipt,
    /// Whether this request changed current delegation.
    pub disposition: DetachDisposition,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parent() -> SessionRef {
        SessionRef {
            node: crate::NodeId::random(),
            session: SessionId::random(),
        }
    }
    #[test]
    fn detach_changes_current_parent_but_never_creator_provenance() {
        let p = parent();
        let creator = Creator::Agent {
            node: p.node,
            session: p.session,
        };
        assert_eq!(Parentage::of(&creator, None).unwrap().parent(), Some(p));
        let receipt = DetachReceipt::new(p, DateTime::UNIX_EPOCH);
        assert_eq!(
            Parentage::of(&creator, Some(receipt)).unwrap(),
            Parentage::Detached(receipt)
        );
        assert_eq!(
            Parentage::of(&creator, Some(receipt)).unwrap().parent(),
            None
        );
        assert_eq!(
            creator,
            Creator::Agent {
                node: p.node,
                session: p.session
            }
        );
    }
    #[test]
    fn a_never_child_and_a_different_creator_cannot_use_a_receipt() {
        let receipt = DetachReceipt::new(parent(), DateTime::UNIX_EPOCH);
        for creator in [
            Creator::Owner,
            Creator::Device {
                id: crate::DeviceId::random(),
            },
            Creator::Agent {
                node: crate::NodeId::random(),
                session: SessionId::random(),
            },
        ] {
            assert_eq!(
                Parentage::of(&creator, Some(receipt)),
                Err(InvalidParentage)
            );
        }
        assert_eq!(Parentage::of(&Creator::Owner, None), Ok(Parentage::Root));
    }
    #[test]
    fn the_receipt_round_trips_complete_cross_node_identity_and_original_time() {
        let receipt = DetachReceipt::new(parent(), DateTime::UNIX_EPOCH);
        let text = serde_json::to_string(&receipt).unwrap();
        let decoded: DetachReceipt = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, receipt);
        assert_eq!(decoded.detached_at(), DateTime::UNIX_EPOCH);
        assert_eq!(decoded.former_parent(), receipt.former_parent());
    }
}
