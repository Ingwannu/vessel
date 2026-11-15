//! Workspace identity and durable placement, independent of sessions and cwd aliases.

use crate::{
    ProjectId, SessionTitle, WorkspaceId, WorkspaceOperationId, WorktreeId,
    workspace::{SetupChoice, WorktreeBranch},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Monotonic registry version; checked advancement refuses exhausted revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct RegistryRevision(u64);
impl RegistryRevision {
    /// Initial persisted version for a newly-created workspace.
    pub const ZERO: Self = Self(0);
    /// Advance without wrapping, which would make an old response current again.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
    /// Ordering/transport code can inspect the unit without mutating it.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for RegistryRevision {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// A background workspace remains durable but is hidden by default lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceVisibility {
    /// User-visible workspace.
    #[default]
    Foreground,
    /// Explicitly requested orchestration/background workspace.
    Background,
}

/// Lists do not infer background inclusion from an unrelated client preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceVisibilityFilter {
    /// Default home/sidebar list.
    #[default]
    Foreground,
    /// Explicit opt-in includes durable background workspaces too.
    All,
}

/// Pin is durable registry presentation, not a device-local sidebar ordering key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePin {
    /// Ordinary project ordering.
    #[default]
    Unpinned,
    /// A person explicitly pinned this workspace.
    Pinned,
}

/// A create choice is a plan; authoritative backing is returned after success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceIsolation {
    /// Separate workspace identity using the project's existing folder.
    Local,
    /// A managed backing created through existing worktree approval/safety checks.
    Worktree {
        /// Branch-off/existing/snapshot request, never inferred from a path string.
        branch: WorktreeBranch,
        /// Saving/creating never implicitly approves a command set.
        setup: SetupChoice,
    },
}

/// Existing backing identity is separate from the logical workspace identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceBacking {
    /// Unowned local directory is retained on archive.
    Local,
    /// A registry-created worktree, removed only after its last active reference.
    Managed {
        /// Exact backing identity, not a branch or cwd heuristic.
        worktree: WorktreeId,
    },
}

/// Archived records remain inspectable and can be restored through reinspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceLifecycle {
    /// Resources can be launched in this workspace.
    Active,
    /// Lifecycle claim blocks launches while owned resources are being drained.
    Archiving {
        /// Kept durably so interrupted cleanup can be inspected and resumed.
        at: DateTime<Utc>,
    },
    /// Exclusive recovery claim; resources stay archived until backing validation completes.
    Restoring {
        /// Retained recovery intent survives a node restart without duplicate creation.
        at: DateTime<Utc>,
    },
    /// Mutations cannot launch resources until authoritative recovery completes.
    Archived {
        /// Host timestamp used for history ordering.
        at: DateTime<Utc>,
    },
}

/// Every client receives placement from the node; duplicate cwd means distinct owners.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceDescriptor {
    /// Stable independent identity.
    pub id: WorkspaceId,
    /// Registered project on this node.
    pub project: ProjectId,
    /// Authoritative execution folder on this node.
    pub path: PathBuf,
    /// User title or absence; automatic names never replace a manual rename.
    pub title: Option<SessionTitle>,
    /// Automatic work from external sources needs an explicit retained-source release.
    pub automation: super::WorkspaceAutomation,
    /// Pin survives clients and reconnects; no second sidebar preference store.
    #[serde(default)]
    pub pin: WorkspacePin,
    /// Backing ownership used by archive/recovery, never guessed from path shape.
    pub backing: WorkspaceBacking,
    /// Active or archived as persisted by the node.
    pub lifecycle: WorkspaceLifecycle,
    /// Durable visibility independent of route/tab state.
    pub visibility: WorkspaceVisibility,
    /// Creation time for stable list ordering.
    pub created_at: DateTime<Utc>,
    /// Revision permits stale response rejection without comparing timestamps.
    pub revision: RegistryRevision,
}

impl WorkspaceDescriptor {
    /// Existing typed IO boundary; the node still revalidates this root on every call.
    #[must_use]
    pub const fn root(&self) -> crate::workspace::WorkspaceRoot {
        match self.backing {
            WorkspaceBacking::Local => crate::workspace::WorkspaceRoot::Project {
                project: self.project,
            },
            WorkspaceBacking::Managed { worktree } => {
                crate::workspace::WorkspaceRoot::Worktree { worktree }
            }
        }
    }
}

/// Creation receipt identity is retained to recover an uncertain connection result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateWorkspace {
    /// Caller-generated unique operation id; repeated differing input is refused.
    pub operation: WorkspaceOperationId,
    /// Existing project registry authority.
    pub project: ProjectId,
    /// Local does not create a session or duplicate directory.
    pub isolation: WorkspaceIsolation,
    /// Optional first explicit name.
    pub title: Option<SessionTitle>,
    /// None inherits an authenticated agent caller's workspace when present.
    pub visibility: Option<WorkspaceVisibility>,
}

/// Explicit filter keeps archived records out of the default sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceList {
    /// Normal home list.
    #[default]
    Active,
    /// History/recovery browser.
    Archived,
    /// Explicit all-record inspection.
    All,
}

/// List snapshots; no secondary client database or path-based identity.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListWorkspaces {
    /// Narrow to one registered project when provided.
    pub project: Option<ProjectId>,
    /// Default active only.
    #[serde(default)]
    pub lifecycle: WorkspaceList,
    /// Default foreground only.
    #[serde(default)]
    pub visibility: WorkspaceVisibilityFilter,
}

/// Direct identity lookup includes archived records for recovery inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetWorkspace {
    /// Exact host-local workspace identity.
    pub workspace: WorkspaceId,
}

/// Manual rename is independent from filesystem/branch names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RenameWorkspace {
    /// Existing identity.
    pub workspace: WorkspaceId,
    /// None resets the displayed override.
    pub title: Option<SessionTitle>,
}

/// Archive only this workspace's resources, retaining shared backing references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ArchiveWorkspace {
    /// Exact ownership scope, never all sessions sharing a cwd.
    pub workspace: WorkspaceId,
}

/// Bulk archive reports individual outcomes; no cross-workspace atomicity promise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ArchiveWorkspaces {
    /// Decode/service enforces the named batch limit before starting work.
    pub workspaces: Vec<WorkspaceId>,
}

/// Reinspect before recovery; never implicitly resumes archived sessions or setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RestoreWorkspace {
    /// Archived identity survives backing recreation.
    pub workspace: WorkspaceId,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revisions_never_wrap() {
        assert_eq!(RegistryRevision::ZERO.next().unwrap().get(), 1);
        let exhausted: RegistryRevision = serde_json::from_str(&u64::MAX.to_string()).unwrap();
        assert!(exhausted.next().is_none());
    }
    #[test]
    fn list_defaults_are_active_foreground() {
        let list: ListWorkspaces = serde_json::from_str("{}").unwrap();
        assert_eq!(list, ListWorkspaces::default());
        assert_eq!(list.lifecycle, WorkspaceList::Active);
        assert_eq!(list.visibility, WorkspaceVisibilityFilter::Foreground);
    }
}

/// Explicit pin mutation cannot accidentally target all workspaces sharing a cwd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PinWorkspace {
    /// Exact host-local identity.
    pub workspace: WorkspaceId,
    /// Explicit desired state, including unpin.
    pub pin: WorkspacePin,
}
