//! Startup-only worker generations cannot be confused with current live capture I/O.
use super::{CheckpointStoreError as Error, query::text};
use rusqlite::{Connection, params};
use vessel_proto::{
    SessionId, WorktreeId,
    checkpoint::{CHECKPOINT_PAGE_ENTRIES, CaptureId, CaptureKey, CheckpointError, CheckpointId},
};

/// Work left by the previous node process, never a currently running worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrphanedCapture {
    /// Candidate invalidation made the private namespace safe to discard.
    Discard {
        /// Exact persisted generation and boundary.
        key: CaptureKey,
    },
    /// Ready was durable: verify its retained archive before settling escrow.
    Retained {
        /// Exact persisted generation and boundary.
        key: CaptureKey,
        /// Published image; never inferred from private filenames.
        checkpoint: CheckpointId,
    },
}
impl OrphanedCapture {
    /// Immutable identity common to both reconciliation actions.
    #[must_use]
    pub const fn key(self) -> CaptureKey {
        match self {
            Self::Discard { key } | Self::Retained { key, .. } => key,
        }
    }
}
/// Bounded keyset scan; consumers settle only after actual filesystem work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanedPage {
    /// Previous-process captures; current active workers never appear.
    pub captures: Vec<OrphanedCapture>,
    /// Continue after this identity even if one object could not be reconciled.
    pub next: Option<CaptureId>,
}
struct Raw {
    capture: String,
    session: String,
    worktree: String,
    generation: String,
    boundary: String,
    state: String,
    checkpoint: Option<String>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Candidate,
    Ready,
    Unavailable,
}
fn decode(raw: Raw) -> Result<OrphanedCapture, Error> {
    let key = CaptureKey {
        capture: text(&raw.capture)?,
        session: text::<SessionId>(&raw.session)?,
        worktree: text::<WorktreeId>(&raw.worktree)?,
        generation: serde_json::from_str(&raw.generation)?,
        boundary: serde_json::from_str(&raw.boundary)?,
    };
    match serde_json::from_str::<State>(&serde_json::to_string(&raw.state)?)? {
        State::Unavailable => Ok(OrphanedCapture::Discard { key }),
        State::Ready => Ok(OrphanedCapture::Retained {
            key,
            checkpoint: text(&raw.checkpoint.ok_or(CheckpointError::StaleCapture)?)?,
        }),
        State::Candidate => Err(CheckpointError::StaleCapture.into()),
    }
}
pub(super) fn page(
    connection: &Connection,
    after: Option<CaptureId>,
) -> Result<OrphanedPage, Error> {
    let mut statement = connection.prepare("SELECT id,session,worktree,CAST(generation AS TEXT),boundary,state,checkpoint FROM checkpoint_captures WHERE worker='orphaned' AND (?1 IS NULL OR id>?1) ORDER BY id LIMIT ?2")?;
    let limit = i64::try_from(CHECKPOINT_PAGE_ENTRIES.saturating_add(1))
        .map_err(|_large| CheckpointError::TooManyPaths)?;
    let mut rows = statement
        .query_map(params![after.map(|id| id.to_string()), limit], |row| {
            Ok(Raw {
                capture: row.get(0)?,
                session: row.get(1)?,
                worktree: row.get(2)?,
                generation: row.get(3)?,
                boundary: row.get(4)?,
                state: row.get(5)?,
                checkpoint: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let more = rows.len() > CHECKPOINT_PAGE_ENTRIES;
    rows.truncate(CHECKPOINT_PAGE_ENTRIES);
    let captures = rows
        .into_iter()
        .map(decode)
        .collect::<Result<Vec<_>, _>>()?;
    let next = if more {
        captures.last().map(|capture| capture.key().capture)
    } else {
        None
    };
    Ok(OrphanedPage { captures, next })
}
impl crate::Store {
    /// Scan only workers fenced at store startup, never current runtime jobs.
    /// # Errors
    /// Malformed old identities/state or a store read failure.
    pub async fn checkpoint_orphaned_page(
        &self,
        after: Option<CaptureId>,
    ) -> Result<OrphanedPage, crate::StoreError> {
        self.read(move |reader| {
            reader.with(|connection| page(connection, after).map_err(crate::StoreError::from))
        })
        .await
    }
}
