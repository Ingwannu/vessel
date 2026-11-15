//! Durable creation admission/CAS on the existing `SQLite` writer (ADR 0091).
//! Store only normalized intent fingerprints, hashed keys and safe receipts.

mod claims;
mod owner;
mod projection;
use owner::Owner;
#[cfg(test)]
pub(crate) mod tests;

use sha2::{Digest as _, Sha256};
use vessel_proto::creation::{Advance, Key, Kind, Receipt, Revision};
use vessel_proto::{BlobHash, NodeId, SessionId, WorkspaceOperationId};

/// Caller key and operation kind form one namespace; Debug redacts the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    kind: Kind,
    key: Key,
    owner: Owner,
}

impl Identity {
    /// Keys are already validated by the protocol boundary.
    #[must_use]
    pub const fn new(kind: Kind, key: Key) -> Self {
        Self {
            kind,
            key,
            owner: Owner::Person,
        }
    }

    /// Bind an agent's keys only to its authenticated origin parent tuple.
    /// The node derives both ids from `Principal`; request data supplies neither.
    #[must_use]
    pub const fn for_agent(node: NodeId, parent: SessionId, key: Key) -> Self {
        Self {
            kind: Kind::Agent,
            key,
            owner: Owner::Agent { node, parent },
        }
    }

    fn digest(&self) -> BlobHash {
        let mut hash = Sha256::new();
        self.owner.hash(&mut hash);
        hash.update(match self.kind {
            Kind::Workspace => b"workspace".as_slice(),
            Kind::Agent => b"agent".as_slice(),
            Kind::Terminal => b"terminal".as_slice(),
        });
        hash.update([0]);
        hash.update(self.key.as_str().as_bytes());
        BlobHash::from_bytes(hash.finalize().into())
    }
}

/// One complete admission candidate; an existing matching intent keeps its ids.
#[derive(Debug)]
pub struct Admission {
    /// Key namespace, persisted as a digest rather than raw caller text.
    pub identity: Identity,
    /// Reserved identities and normalized intent fingerprint.
    pub receipt: Receipt,
}

/// Writer-owned mutation; service/UI code never executes SQL directly.
#[derive(Debug)]
pub(crate) enum Write {
    Admit(Admission),
    Advance {
        operation: WorkspaceOperationId,
        expected: Revision,
        update: Advance,
    },
}

pub(crate) use projection::{apply, lookup, operation, recover};
