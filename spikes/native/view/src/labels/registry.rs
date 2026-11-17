//! A host's workspaces as the labels see them (ADR 0082, ADR 0086): which
//! workspace a session belongs to, and each workspace's labels in order.
//!
//! [Decision Log]
//! - Purpose: a session row shows its workspace's labels, and its menu's
//!   Labels page assigns to that workspace, as Paseo's sidebar does.
//! - Constraint: assignments live in the registry snapshot's `labels`
//!   projection (protocol 28), keyed by the registry's workspace identity;
//!   a session reaches its workspace only through `memberships`, never by
//!   its directory. The snapshot is replaced whole, by revision.
//! - Choice: keep only those two maps from each snapshot, with its
//!   revision for the next `changed_from` read; a committed assignment
//!   answer updates its workspace at once. A snapshot read before that
//!   commit can arrive after its answer: the assignment is kept over a
//!   snapshot no newer than the revision held when it was answered, and
//!   over one newer snapshot that still disagrees (the commit's own
//!   snapshot follows it), then the registry wins again.
//! - Cost: a session without a membership (an older one) has no
//!   workspace, so no labels and no Labels page.

use std::collections::BTreeMap;

use vessel_proto::registry::{RegistryRevision, RegistrySnapshot, WorkspaceResource};
use vessel_proto::{SessionId, WorkspaceId};

use super::Name;

/// One host's session workspaces and workspace labels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Assignments {
    revision: Option<RegistryRevision>,
    sessions: BTreeMap<SessionId, WorkspaceId>,
    labels: BTreeMap<WorkspaceId, Vec<Name>>,
    /// Committed assignments the registry may not show yet.
    committed: BTreeMap<WorkspaceId, Committed>,
}

/// A committed assignment awaiting the snapshot that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Committed {
    labels: Vec<Name>,
    /// The revision held when its answer came.
    held: Option<RegistryRevision>,
    /// Whether one newer snapshot without it was already passed over.
    passed: bool,
}

impl Assignments {
    /// What `snapshot` says, replacing everything before it.
    #[must_use]
    pub fn read(snapshot: &RegistrySnapshot) -> Self {
        let sessions = snapshot
            .memberships
            .iter()
            .filter_map(|membership| match &membership.resource {
                WorkspaceResource::Session { session } => Some((*session, membership.workspace)),
                WorkspaceResource::Terminal { .. } => None,
            })
            .collect();
        let labels = snapshot
            .labels
            .iter()
            .map(|assigned| (assigned.workspace, assigned.labels.clone()))
            .collect();
        Self {
            revision: Some(snapshot.revision),
            sessions,
            labels,
            committed: BTreeMap::new(),
        }
    }

    /// `snapshot` replaces what was read, keeping a committed assignment
    /// over a snapshot that may predate its commit.
    pub fn refreshed(&mut self, snapshot: &RegistrySnapshot) {
        let mut next = Self::read(snapshot);
        for (workspace, committed) in std::mem::take(&mut self.committed) {
            if next.labels(workspace) == committed.labels.as_slice() {
                continue;
            }
            let older = committed.held.is_some_and(|held| snapshot.revision <= held);
            if !older && committed.passed {
                continue;
            }
            next.set(workspace, committed.labels.clone());
            let _kept = next.committed.insert(
                workspace,
                Committed {
                    passed: committed.passed || !older,
                    ..committed
                },
            );
        }
        *self = next;
    }

    /// The snapshot's revision, for the next read that waits for a change.
    #[must_use]
    pub const fn revision(&self) -> Option<RegistryRevision> {
        self.revision
    }

    /// The workspace `session` belongs to.
    #[must_use]
    pub fn workspace(&self, session: SessionId) -> Option<WorkspaceId> {
        self.sessions.get(&session).copied()
    }

    /// `workspace`'s labels, in their committed order.
    #[must_use]
    pub fn labels(&self, workspace: WorkspaceId) -> &[Name] {
        self.labels.get(&workspace).map_or(&[], Vec::as_slice)
    }

    /// A committed assignment: `workspace`'s labels are now `labels`.
    pub fn assigned(&mut self, workspace: WorkspaceId, labels: Vec<Name>) {
        self.set(workspace, labels.clone());
        let _previous = self.committed.insert(
            workspace,
            Committed {
                labels,
                held: self.revision,
                passed: false,
            },
        );
    }

    fn set(&mut self, workspace: WorkspaceId, labels: Vec<Name>) {
        if labels.is_empty() {
            let _unlabelled = self.labels.remove(&workspace);
        } else {
            let _previous = self.labels.insert(workspace, labels);
        }
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
