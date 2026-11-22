//! A making run pins its exact workspace and session before provisioning.
//! Runs are trimmed and schedules can be deleted while making, so cleanup
//! holds this independent bounded row until its effects are reconciled.

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::automation::{AfterRun, RunNumber};
use vessel_proto::{ScheduleId, SessionId, WorkspaceId};

use crate::StoreError;

/// The registered actor and deferred archive state are distinct from the run's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunBindingStage {
    /// The workspace and session may not yet exist; inspect the durable receipts.
    Bound,
    /// The exact associated session was registered.
    Started,
    /// The turn ended and the person can release this Keep receipt.
    Kept,
    /// The session had queued work or an in-flight turn during automatic archive.
    DeferredBusy,
    /// Another live member still has work in the same workspace.
    DeferredMembers,
    /// The caller released its pin after proving there were no effects.
    RetiringUnpublished,
    /// The caller released its pin after proving the backing was removed.
    RetiringArchive,
    /// Restart cannot yet prove the old process lifetime has ended.
    Unconfirmed,
}

impl RunBindingStage {
    const fn sql(self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Started => "started",
            Self::Kept => "kept",
            Self::DeferredBusy => "deferred_busy",
            Self::DeferredMembers => "deferred_members",
            Self::RetiringUnpublished => "retiring_unpublished",
            Self::RetiringArchive => "retiring_archive",
            Self::Unconfirmed => "unconfirmed",
        }
    }

    /// A terminal binding remains only to deliver an idempotent retirement.
    #[must_use]
    pub const fn retirement(self) -> Option<crate::ProvisionRetirement> {
        match self {
            Self::RetiringUnpublished => Some(crate::ProvisionRetirement::Unpublished),
            Self::RetiringArchive => Some(crate::ProvisionRetirement::CleanArchive),
            Self::Bound
            | Self::Started
            | Self::Kept
            | Self::DeferredBusy
            | Self::DeferredMembers
            | Self::Unconfirmed => None,
        }
    }

    fn read(value: &str) -> Result<Self, StoreError> {
        match value {
            "bound" => Ok(Self::Bound),
            "started" => Ok(Self::Started),
            "kept" => Ok(Self::Kept),
            "deferred_busy" => Ok(Self::DeferredBusy),
            "deferred_members" => Ok(Self::DeferredMembers),
            "retiring_unpublished" => Ok(Self::RetiringUnpublished),
            "retiring_archive" => Ok(Self::RetiringArchive),
            "unconfirmed" => Ok(Self::Unconfirmed),
            _ => Err(StoreError::RegistryConflict),
        }
    }
}

/// A run pin is independent of the schedule record and its retention horizon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleRunBinding {
    /// The schedule that admitted the run.
    pub schedule: ScheduleId,
    /// The already recorded run.
    pub run: RunNumber,
    /// Allocated before the producer reserves it.
    pub workspace: WorkspaceId,
    /// Allocated before the session start lease.
    pub session: SessionId,
    /// The rule the run captured at admission, independent of later edits.
    pub after: AfterRun,
    /// How far the run has gone, or why its archive could not finish.
    pub stage: RunBindingStage,
}

pub(super) fn reserve(
    connection: &Connection,
    binding: ScheduleRunBinding,
) -> Result<(), StoreError> {
    if binding.stage != RunBindingStage::Bound {
        return Err(StoreError::RegistryConflict);
    }
    if let Some(existing) = find(connection, binding.schedule, binding.run)? {
        return if existing == binding {
            Ok(())
        } else {
            Err(StoreError::RegistryConflict)
        };
    }
    let count: i64 =
        connection.query_row("SELECT COUNT(*) FROM schedule_run_bindings", [], |row| {
            row.get(0)
        })?;
    let limit = i64::try_from(crate::MAX_WORKSPACES).map_err(StoreError::RegistryRevisionRange)?;
    if count >= limit {
        return Err(StoreError::RegistryFull);
    }
    let making: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schedule_runs WHERE schedule=?1 AND number=?2
             AND json_extract(run,'$.outcome.outcome')='making')",
        params![binding.schedule.to_string(), binding.run.get()],
        |row| row.get(0),
    )?;
    if !making {
        return Err(StoreError::RegistryConflict);
    }
    let after = match binding.after {
        AfterRun::Archive => "archive",
        AfterRun::Keep => "keep",
        AfterRun::Unknown => return Err(StoreError::RegistryConflict),
    };
    let saved = connection.execute(
        "INSERT INTO schedule_run_bindings (schedule,number,workspace,session,after,stage)
         VALUES (?1,?2,?3,?4,?5,'bound') ON CONFLICT DO NOTHING",
        params![
            binding.schedule.to_string(),
            binding.run.get(),
            binding.workspace.to_string(),
            binding.session.to_string(),
            after
        ],
    )?;
    if saved != 1 {
        return Err(StoreError::RegistryConflict);
    }
    Ok(())
}

pub(super) fn advance(
    connection: &Connection,
    previous: ScheduleRunBinding,
    stage: RunBindingStage,
) -> Result<(), StoreError> {
    if stage == RunBindingStage::Bound
        || previous.stage.retirement().is_some()
        || find(connection, previous.schedule, previous.run)? != Some(previous)
    {
        return Err(StoreError::RegistryConflict);
    }
    let _written = connection.execute(
        "UPDATE schedule_run_bindings SET stage=?3 WHERE schedule=?1 AND number=?2",
        params![
            previous.schedule.to_string(),
            previous.run.get(),
            stage.sql()
        ],
    )?;
    Ok(())
}

pub(super) fn finish(
    connection: &Connection,
    binding: ScheduleRunBinding,
) -> Result<(), StoreError> {
    if find(connection, binding.schedule, binding.run)? != Some(binding) {
        return Err(StoreError::RegistryConflict);
    }
    let _deleted = connection.execute(
        "DELETE FROM schedule_run_bindings WHERE schedule=?1 AND number=?2",
        params![binding.schedule.to_string(), binding.run.get()],
    )?;
    Ok(())
}

pub(crate) fn find(
    connection: &Connection,
    schedule: ScheduleId,
    run: RunNumber,
) -> Result<Option<ScheduleRunBinding>, StoreError> {
    let row = connection.query_row(
        "SELECT schedule, number, workspace, session, after, stage FROM schedule_run_bindings WHERE schedule=?1 AND number=?2",
        params![schedule.to_string(), run.get()],
        read_row,
    ).optional()?;
    row.map(decode).transpose()
}

pub(crate) fn load(connection: &Connection) -> Result<Vec<ScheduleRunBinding>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT schedule, number, workspace, session, after, stage FROM schedule_run_bindings ORDER BY schedule,number",
    )?;
    let rows = statement.query_map([], read_row)?;
    rows.map(|row| decode(row?)).collect()
}

/// Called only by a person's Archive mark in the same writer savepoint.
pub(crate) fn request_keep_release(
    connection: &Connection,
    session: SessionId,
) -> Result<bool, StoreError> {
    let changed = connection.execute(
        "UPDATE schedule_run_bindings SET release_requested=1
         WHERE session=?1 AND after='keep' AND stage IN ('bound','started','kept') AND release_requested=0",
        [session.to_string()],
    )?;
    Ok(changed != 0)
}

pub(crate) fn keeps_session(
    connection: &Connection,
    session: SessionId,
    requested: bool,
) -> Result<bool, StoreError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schedule_run_bindings
         WHERE session=?1 AND after='keep' AND stage IN ('bound','started','kept','deferred_busy','deferred_members','unconfirmed')
         AND (NOT ?2 OR release_requested=1))",
        params![session.to_string(), requested],
        |row| row.get(0),
    )?)
}

pub(crate) fn awaiting_archive(
    connection: &Connection,
) -> Result<Vec<ScheduleRunBinding>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT schedule, number, workspace, session, after, stage FROM schedule_run_bindings
         WHERE stage IN ('deferred_busy','deferred_members')
         OR (stage='kept' AND after='keep' AND release_requested=1)
         ORDER BY schedule,number",
    )?;
    let rows = statement.query_map([], read_row)?;
    rows.map(|row| decode(row?)).collect()
}

type Row = (String, u32, String, String, String, String);

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn decode(
    (schedule, run, workspace, session, after, stage): Row,
) -> Result<ScheduleRunBinding, StoreError> {
    Ok(ScheduleRunBinding {
        schedule: schedule.parse().map_err(|source| StoreError::CorruptId {
            text: schedule,
            source,
        })?,
        run: RunNumber::new(run).ok_or(StoreError::RegistryConflict)?,
        workspace: workspace.parse().map_err(|source| StoreError::CorruptId {
            text: workspace,
            source,
        })?,
        session: crate::codec::session_id_from_sql(&session)?,
        after: match after.as_str() {
            "archive" => AfterRun::Archive,
            "keep" => AfterRun::Keep,
            _ => return Err(StoreError::RegistryConflict),
        },
        stage: RunBindingStage::read(&stage)?,
    })
}
