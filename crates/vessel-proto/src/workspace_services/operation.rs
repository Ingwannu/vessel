//! Optimistic mutation fences are data, never approval or start-lease authority.

use serde::{Deserialize, Serialize};

use crate::{
    ProtocolVersion, WorkspaceId,
    workspace::{CommandsDigest, ProjectCommands},
};

use super::{ServiceRefusal, ServiceSnapshotId, WorkspaceScriptRef, WorkspaceScriptRow};

/// Start still rechecks canonical membership, automation, approval and a lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct StartWorkspaceScript {
    /// Never infer workspace identity from the legacy script placement or cwd.
    pub target: WorkspaceScriptRef,
    /// The state the person inspected, not an actual-run identity.
    pub snapshot: ServiceSnapshotId,
    /// An optimistic complete-definition fence, not a new approval.
    pub definition: CommandsDigest,
}

/// Stop must join actual owned cleanup; its request cannot assert OS exit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct StopWorkspaceScript {
    /// The node resolves the canonical current resource/run itself.
    pub target: WorkspaceScriptRef,
    /// A stale UI cannot silently stop a replacement observation.
    pub snapshot: ServiceSnapshotId,
}

/// Restart requires actual prior-run join plus the canonical restart-lease CAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct RestartWorkspaceScript {
    /// Stable workspace/name is not a client-selected process id.
    pub target: WorkspaceScriptRef,
    /// A metadata fence does not substitute for the exact prior start lease.
    pub snapshot: ServiceSnapshotId,
    /// Approval remains the existing complete command-set authorization.
    pub definition: CommandsDigest,
}

/// Admission acknowledgement is distinct from healthy/listening/cleanup proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceScriptOutcome {
    /// The tracked operation was admitted, not proved healthy or fully stopped.
    Accepted {
        /// The explicit canonical identity whose operation was admitted.
        workspace: WorkspaceId,
        /// Reconcile a lost response through observation, never blind replay.
        snapshot: ServiceSnapshotId,
        /// Descriptive supervised state only.
        row: Box<WorkspaceScriptRow>,
    },
    /// No new operation was necessary for this current observation.
    Already {
        /// Never alias two workspaces that share a directory.
        workspace: WorkspaceId,
        /// The current metadata etag.
        snapshot: ServiceSnapshotId,
        /// The current configured/captured definition and terminal observation.
        row: Box<WorkspaceScriptRow>,
    },
    /// The existing approval display must show or refuse every approved field.
    NeedsApproval {
        /// A new status API cannot create a workspace from a path.
        workspace: WorkspaceId,
        /// Reuse the complete approval owner, not a partial row digest.
        commands: ProjectCommands,
    },
    /// Scope/principal/protocol checks run before any lookup or domain response.
    Refused {
        /// A checked local registry target, not process authority.
        workspace: WorkspaceId,
        /// Safe domain classification, never raw helper/process error text.
        reason: ServiceRefusal,
    },
    /// An unrecognized reply is never a successful start/restart/stop.
    #[serde(other)]
    Unknown,
}

impl WorkspaceScriptOutcome {
    /// All direct/nested mutation replies reuse the complete approval gate.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        match self {
            Self::NeedsApproval {
                workspace,
                commands,
            } => commands
                .for_protocol(protocol)
                .map(|commands| Self::NeedsApproval {
                    workspace: *workspace,
                    commands,
                }),
            Self::Accepted {
                workspace: _,
                snapshot: _,
                row: _,
            }
            | Self::Already {
                workspace: _,
                snapshot: _,
                row: _,
            }
            | Self::Refused {
                workspace: _,
                reason: _,
            }
            | Self::Unknown => None,
        }
    }
}
