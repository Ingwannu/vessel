//! Archive admission belongs to an exact token. Metadata may change while
//! actors drain, but only that token may publish a lifecycle transition.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::registry::{RegistryRevision, WorkspaceBacking, WorkspaceLifecycle};
use vessel_proto::{WorkspaceId, WorkspaceOperationId, WorktreeId};

use super::{ReservationStage, project_reference, read as registry, write};
use crate::StoreError;

/// The caller's authority to stop admitted workspace members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceArchivePurpose {
    /// A person asked to archive all exact members.
    Explicit,
    /// Background cleanup must honor live members and queue safeguards.
    Automatic,
}

impl WorkspaceArchivePurpose {
    const fn sql(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Automatic => "automatic",
        }
    }
}

/// A deferred automatic archive no longer owns its Active workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveDefer {
    /// The scheduled session is running or has queued work.
    BusySession,
    /// At least one other explicit member still lives.
    OtherMembers,
}

impl ArchiveDefer {
    const fn sql(self) -> &'static str {
        match self {
            Self::BusySession => "busy_session",
            Self::OtherMembers => "other_members",
        }
    }
}

/// Authority reserved before a cleanup effect can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceArchiveClaim {
    /// Exactly one independently provisioned workspace.
    pub workspace: WorkspaceId,
    /// A fresh node-minted claim token; stale effects cannot settle it.
    pub token: WorkspaceOperationId,
    /// Changes to this policy require a fresh admission.
    pub purpose: WorkspaceArchivePurpose,
}

/// Current internal archive ownership or the reason a former claim deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryArchive {
    /// The last issued token, which is inactive if deferred.
    pub claim: WorkspaceArchiveClaim,
    /// An Active workspace can be claimed again with a fresh token.
    pub deferred: Option<ArchiveDefer>,
}

/// Which transition an archive claimant proved safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveSettlement {
    /// Actors are settled. Physical backing debt may still be retained.
    Archived {
        /// Host time for the archived history.
        at: DateTime<Utc>,
    },
    /// No longer own admission, until later activity warrants a new claim.
    Deferred(ArchiveDefer),
}

pub(crate) fn read(
    connection: &Connection,
    workspace: WorkspaceId,
) -> Result<Option<RegistryArchive>, StoreError> {
    let stored: Option<(String, String, Option<String>)> = connection
        .query_row(
            "SELECT token,purpose,defer FROM workspace_archive_claims WHERE workspace=?1",
            [workspace.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    stored
        .map(|(token, purpose, deferred)| {
            let token = token.parse().map_err(|source| StoreError::CorruptId {
                text: token,
                source,
            })?;
            let purpose = match purpose.as_str() {
                "explicit" => WorkspaceArchivePurpose::Explicit,
                "automatic" => WorkspaceArchivePurpose::Automatic,
                _ => return Err(StoreError::RegistryConflict),
            };
            let deferred = match deferred.as_deref() {
                None => None,
                Some("busy_session") => Some(ArchiveDefer::BusySession),
                Some("other_members") => Some(ArchiveDefer::OtherMembers),
                Some(_) => return Err(StoreError::RegistryConflict),
            };
            Ok(RegistryArchive {
                claim: WorkspaceArchiveClaim {
                    workspace,
                    token,
                    purpose,
                },
                deferred,
            })
        })
        .transpose()
}

pub(super) fn claim(
    connection: &Connection,
    claim: WorkspaceArchiveClaim,
    expected: RegistryRevision,
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    let workspace = claim.workspace;
    let mut record = registry::workspace(connection, workspace)?
        .ok_or(StoreError::RegistryMissing(workspace))?;
    if record.descriptor.revision != expected
        || record.descriptor.lifecycle != WorkspaceLifecycle::Active
        || !registry::reservation(connection, workspace)?
            .is_some_and(|value| value.stage == ReservationStage::Ready)
    {
        return Err(StoreError::RegistryConflict);
    }
    let changed = connection.execute(
        "INSERT INTO workspace_archive_claims (workspace,token,purpose,defer) VALUES (?1,?2,?3,NULL)
         ON CONFLICT(workspace) DO UPDATE SET token=excluded.token,purpose=excluded.purpose,defer=NULL
         WHERE workspace_archive_claims.defer IS NOT NULL AND workspace_archive_claims.token!=excluded.token",
        params![workspace.to_string(), claim.token.to_string(), claim.purpose.sql()],
    )?;
    if changed != 1 {
        return Err(StoreError::RegistryConflict);
    }
    record.descriptor.lifecycle = WorkspaceLifecycle::Archiving { at };
    record.descriptor.revision = expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    write::put(connection, &record, Some(expected))
}

fn is_active_claim(
    connection: &Connection,
    claim: WorkspaceArchiveClaim,
) -> Result<bool, StoreError> {
    Ok(read(connection, claim.workspace)?
        == Some(RegistryArchive {
            claim,
            deferred: None,
        }))
}

pub(super) fn settle(
    connection: &Connection,
    claim: WorkspaceArchiveClaim,
    settlement: ArchiveSettlement,
) -> Result<(), StoreError> {
    let workspace = claim.workspace;
    let mut record = registry::workspace(connection, workspace)?
        .ok_or(StoreError::RegistryMissing(workspace))?;
    if !matches!(
        record.descriptor.lifecycle,
        WorkspaceLifecycle::Archiving { .. }
    ) || !is_active_claim(connection, claim)?
    {
        return Err(StoreError::RegistryConflict);
    }
    let expected = record.descriptor.revision;
    record.descriptor.lifecycle = match settlement {
        ArchiveSettlement::Archived { at } => {
            let _deleted = connection.execute(
                "DELETE FROM workspace_archive_claims WHERE workspace=?1",
                [workspace.to_string()],
            )?;
            WorkspaceLifecycle::Archived { at }
        }
        ArchiveSettlement::Deferred(reason)
            if claim.purpose == WorkspaceArchivePurpose::Automatic =>
        {
            let _updated = connection.execute(
                "UPDATE workspace_archive_claims SET defer=?2 WHERE workspace=?1",
                params![workspace.to_string(), reason.sql()],
            )?;
            WorkspaceLifecycle::Active
        }
        ArchiveSettlement::Deferred(_) => return Err(StoreError::RegistryConflict),
    };
    record.descriptor.revision = expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    write::put(connection, &record, Some(expected))
}

pub(crate) fn owns_backing(
    connection: &Connection,
    claim: WorkspaceArchiveClaim,
    worktree: WorktreeId,
) -> Result<bool, StoreError> {
    let transaction = connection.unchecked_transaction()?;
    let owns = is_active_claim(&transaction, claim)?
        && registry::workspace(&transaction, claim.workspace)?.is_some_and(|record| {
            matches!(
                record.descriptor.lifecycle,
                WorkspaceLifecycle::Archiving { .. }
            ) && record.descriptor.backing == (WorkspaceBacking::Managed { worktree })
        })
        && registry::reservation(&transaction, claim.workspace)?.is_some_and(|reservation| {
            reservation.stage == ReservationStage::Ready && reservation.worktree == Some(worktree)
        })
        && project_reference::worktree(&transaction, worktree, Some(claim.workspace))?.is_none();
    transaction.commit()?;
    Ok(owns)
}
