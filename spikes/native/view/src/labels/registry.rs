//! A host's workspaces as the labels see them (ADR 0082, ADR 0086): which
//! workspace a session belongs to, and each workspace's labels in order.
//!
//! [Decision Log]
//! - Purpose: a session row shows its workspace's labels, and its menu's
//!   Labels page assigns to that workspace, as Paseo's sidebar does.
//! - Constraint: assignments live in the registry snapshot's `labels`
//!   projection (protocol 28), keyed by the registry's workspace identity;
//!   a session reaches its workspace through the registry, never by its
//!   directory. The snapshot is replaced whole, by revision. A session
//!   placed in a project (its root or a folder, not a worktree) is a
//!   derived member (ADR 0106 choice 4): the snapshot lists it, but its
//!   start writes nothing, so a read waiting on the revision does not wake
//!   for it and a new session would wait up to a whole wait for its row's
//!   Labels.
//! - Choice: keep only those maps from each snapshot, with its revision
//!   for the next `changed_from` read. A session without an explicit row
//!   takes the node's own derived rule from its `SessionInfo.workspace`
//!   project, as the ADR tells a client to: the project's oldest Active,
//!   foreground `Local` workspace. A committed assignment
//!   answer updates its workspace at once. A snapshot read before that
//!   commit can arrive after its answer: the assignment is kept over a
//!   snapshot no newer than the revision held when it was answered, and
//!   over one newer snapshot that still disagrees (the commit's own
//!   snapshot follows it), then the registry wins again.
//! - Cost: the rule is the node's, kept twice (vessel-store
//!   `derived_memberships`); a change there changes this. A session in a
//!   worktree, or with no project (a plain folder), has no workspace, so
//!   no labels and no Labels page, as on the node.

use std::collections::BTreeMap;

use vessel_proto::registry::{
    RegistryRevision, RegistrySnapshot, WorkspaceBacking, WorkspaceLifecycle, WorkspaceResource,
    WorkspaceVisibility,
};
use vessel_proto::{ProjectId, SessionId, WorkspaceId};

use super::Name;

/// One host's session workspaces and workspace labels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Assignments {
    revision: Option<RegistryRevision>,
    sessions: BTreeMap<SessionId, WorkspaceId>,
    /// Each project's workspace for its derived members.
    owners: BTreeMap<ProjectId, WorkspaceId>,
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
            owners: owners(snapshot),
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

    /// The workspace `session` belongs to: its explicit row, else, placed
    /// in `project` (its `SessionInfo.workspace` without a worktree, not
    /// archived), that project's workspace.
    #[must_use]
    pub fn workspace(&self, session: SessionId, project: Option<ProjectId>) -> Option<WorkspaceId> {
        self.sessions
            .get(&session)
            .copied()
            .or_else(|| project.and_then(|project| self.owners.get(&project).copied()))
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

/// Each project's oldest Active, foreground `Local` workspace (by creation,
/// then identity): the node's `derived_memberships` owner.
fn owners(snapshot: &RegistrySnapshot) -> BTreeMap<ProjectId, WorkspaceId> {
    let mut oldest = BTreeMap::new();
    for descriptor in &snapshot.workspaces {
        let live = descriptor.lifecycle == WorkspaceLifecycle::Active
            && descriptor.visibility == WorkspaceVisibility::Foreground
            && descriptor.backing == WorkspaceBacking::Local;
        if !live {
            continue;
        }
        let candidate = (descriptor.created_at, descriptor.id);
        let _owner = oldest
            .entry(descriptor.project)
            .and_modify(|owner| {
                if candidate < *owner {
                    *owner = candidate;
                }
            })
            .or_insert(candidate);
    }
    oldest
        .into_iter()
        .map(|(project, (_, workspace))| (project, workspace))
        .collect()
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
