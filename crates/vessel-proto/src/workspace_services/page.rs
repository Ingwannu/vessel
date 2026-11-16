//! Latest-only page values, not retained snapshot storage or workspace authority.

use serde::{Deserialize, Serialize};

use crate::workspace::{CommandsDigest, MAX_PROJECT_COMMANDS, PROJECT_FILE_KEYS};
use crate::{ProtocolVersion, WorkspaceId, scripts::ScriptName};

use super::{ServiceSnapshotId, ServiceUnavailable, WorkspaceScriptRow};

/// A client must not convert unknown approval into permission to run commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ServiceApproval {
    /// The node currently recognizes this exact digest's authorization.
    Approved,
    /// A person must inspect and approve the existing complete command set.
    Unapproved,
    /// No automatic operation follows an unrecognized state.
    #[serde(other)]
    Unknown,
}

/// Detailed command display stays in the existing complete approval owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceConfiguration {
    /// An authoritative read found no configuration file.
    None,
    /// A digest fence is not execution authority and must be peer-projected.
    Current {
        /// The exact complete command set, not only the displayed row.
        digest: CommandsDigest,
        /// The node's current authorization observation.
        approval: ServiceApproval,
    },
    /// A malformed whole file cannot become an empty successful plan.
    Invalid,
    /// Missing sources must be visible rather than guessed as no file.
    Unavailable {
        /// Keep raw parser/private path text out of status.
        reason: ServiceUnavailable,
    },
    /// A future configuration state cannot enable a control.
    #[serde(other)]
    Unknown,
}

impl ServiceConfiguration {
    /// A compact summary cannot prove which approved fields were displayed.
    /// Hide its digest conservatively until the complete display gate admits it.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        match self {
            Self::Current {
                digest: _,
                approval: _,
            } => PROJECT_FILE_KEYS
                .is_none_or(|since| protocol < since)
                .then_some(Self::Unavailable {
                    reason: ServiceUnavailable::ApprovalRequired,
                }),
            Self::None | Self::Invalid | Self::Unavailable { reason: _ } | Self::Unknown => None,
        }
    }
}

/// A cursor selects only within the request's canonical workspace/snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceCursor {
    /// A changed snapshot makes the cursor stale, never retained history.
    pub snapshot: ServiceSnapshotId,
    /// Stable bytewise name ordering, independent of the client's locale.
    pub after: ScriptName,
}

/// Private fields preserve count, ordering and continuation consistency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawServicePage")]
pub struct ServicePage {
    workspace: WorkspaceId,
    snapshot: ServiceSnapshotId,
    configuration: ServiceConfiguration,
    rows: Vec<WorkspaceScriptRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<ServiceCursor>,
}

/// Every page field crosses the borrowed display boundary explicitly.
#[derive(Debug, Clone, Copy)]
pub struct ServicePageView<'a> {
    /// This is an explicit registry identity, not a directory association.
    pub workspace: WorkspaceId,
    /// Metadata etag, never an actual-run or incarnation token.
    pub snapshot: ServiceSnapshotId,
    /// Full approval still uses the existing commands display.
    pub configuration: &'a ServiceConfiguration,
    /// Bytewise sorted names; the view can sort its own presentation separately.
    pub rows: &'a [WorkspaceScriptRow],
    /// No retained archive is implied by a continuation.
    pub next: Option<&'a ServiceCursor>,
}

/// Malformed pages refuse rather than reorder, truncate or merge snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidServicePage {
    /// Reuse the canonical project-command bound without a numeric copy.
    #[error("a service page has at most {max} entries", max = MAX_PROJECT_COMMANDS)]
    TooMany,
    /// Equal and descending names would make cursors ambiguous.
    #[error("service page names are unique and strictly ordered")]
    Order,
    /// A continuation must identify this snapshot and its final entry.
    #[error("the service page continuation does not match its snapshot and last row")]
    Cursor,
}

impl ServicePage {
    /// Validate a projection, without attesting physical membership or ownership.
    ///
    /// # Errors
    /// A page over its bound, unordered/duplicate names, or invalid continuation.
    pub fn new(
        workspace: WorkspaceId,
        snapshot: ServiceSnapshotId,
        configuration: ServiceConfiguration,
        rows: Vec<WorkspaceScriptRow>,
        next: Option<ServiceCursor>,
    ) -> Result<Self, InvalidServicePage> {
        if rows.len() > MAX_PROJECT_COMMANDS {
            return Err(InvalidServicePage::TooMany);
        }
        for (left, right) in rows.iter().zip(rows.iter().skip(1)) {
            if left.view().definition.name >= right.view().definition.name {
                return Err(InvalidServicePage::Order);
            }
        }
        if let Some(cursor) = &next
            && (cursor.snapshot != snapshot
                || rows
                    .last()
                    .is_none_or(|row| row.view().definition.name != cursor.after))
        {
            return Err(InvalidServicePage::Cursor);
        }
        Ok(Self {
            workspace,
            snapshot,
            configuration,
            rows,
            next,
        })
    }

    /// Preserve complete presentation without permitting independent mutations.
    #[must_use]
    pub fn view(&self) -> ServicePageView<'_> {
        let Self {
            workspace,
            snapshot,
            configuration,
            rows,
            next,
        } = self;
        ServicePageView {
            workspace: *workspace,
            snapshot: *snapshot,
            configuration,
            rows,
            next: next.as_ref(),
        }
    }

    /// Every direct/nested reply must retain the existing approval display gate.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        self.configuration
            .for_protocol(protocol)
            .map(|configuration| {
                let mut page = self.clone();
                page.configuration = configuration;
                page
            })
    }
}

#[derive(Deserialize)]
struct RawServicePage {
    workspace: WorkspaceId,
    snapshot: ServiceSnapshotId,
    configuration: ServiceConfiguration,
    rows: Vec<WorkspaceScriptRow>,
    #[serde(default)]
    next: Option<ServiceCursor>,
}

impl TryFrom<RawServicePage> for ServicePage {
    type Error = InvalidServicePage;

    fn try_from(raw: RawServicePage) -> Result<Self, Self::Error> {
        Self::new(
            raw.workspace,
            raw.snapshot,
            raw.configuration,
            raw.rows,
            raw.next,
        )
    }
}

/// Queries never create a snapshot archive or a workspace through cwd inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceQuery {
    /// Obtain the current bounded first page without long polling.
    Initial,
    /// Wait only within the qualified shared and node-wide waiter budgets.
    After {
        /// Replaced observations respond immediately rather than wait.
        snapshot: ServiceSnapshotId,
    },
    /// Read only the same latest snapshot, otherwise return stale.
    Page {
        /// A name cursor does not keep an older observation alive.
        cursor: ServiceCursor,
    },
}

/// No field can request helper work or reserve a port on this Read boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct WorkspaceScriptQuery {
    /// Canonical identity is validated by the node, not a path guess.
    pub workspace: WorkspaceId,
    /// A closed typed query choice, never an arbitrary wait duration.
    pub query: ServiceQuery,
}

/// A missing source is not an empty successful script list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceScriptQueryResult {
    /// A single consistent bounded page.
    Snapshot {
        /// This owner enforces cursor/name/count invariants.
        page: ServicePage,
    },
    /// The same observation remains current after the bounded wait.
    NotChanged {
        /// Do not confuse responses for different workspaces.
        workspace: WorkspaceId,
        /// This is metadata only.
        snapshot: ServiceSnapshotId,
    },
    /// Typed missing producers cannot be mistaken for no scripts.
    Unavailable {
        /// The requested canonical workspace, not an inferred one.
        workspace: WorkspaceId,
        /// No raw private error text crosses this view.
        reason: ServiceUnavailable,
    },
    /// No retained old snapshot is recreated for a cursor.
    Stale {
        /// The request's explicit registry identity.
        workspace: WorkspaceId,
        /// Absence is not a sentinel etag or permission to replay.
        current: Option<ServiceSnapshotId>,
    },
    /// A future reply never acknowledges a successful empty plan.
    #[serde(other)]
    Unknown,
}

impl WorkspaceScriptQueryResult {
    /// The shared reply adapter invokes this for direct and nested snapshots.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        match self {
            Self::Snapshot { page } => page
                .for_protocol(protocol)
                .map(|page| Self::Snapshot { page }),
            Self::NotChanged {
                workspace: _,
                snapshot: _,
            }
            | Self::Unavailable {
                workspace: _,
                reason: _,
            }
            | Self::Stale {
                workspace: _,
                current: _,
            }
            | Self::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests;
