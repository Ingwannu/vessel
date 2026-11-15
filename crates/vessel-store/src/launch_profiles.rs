//! Whole-list launch-profile storage; no second file, live cache or credentials.
//!
//! Compare-and-replace runs in the existing writer's savepoint and its
//! answer waits for the outer commit. Reads fetch the revision and bounded
//! body in one statement, so a concurrent writer cannot split the snapshot.

use crate::StoreError;
use rusqlite::{Connection, params};
use vessel_proto::launch_profiles::{
    LaunchPayload, LaunchProfileRevision, LaunchProfiles, LaunchProfilesSnapshot,
};

/// Outcome of a whole-list CAS, acknowledged only after the batch commits.
#[derive(Debug)]
#[must_use]
pub enum LaunchProfilesChange {
    /// Matching revision and identical list: revision did not advance.
    Unchanged(LaunchProfilesSnapshot),
    /// Complete replacement, in the submitted order.
    Replaced(LaunchProfilesSnapshot),
    /// The caller must reread before deciding what to replace.
    Stale {
        /// Current committed revision.
        current: LaunchProfileRevision,
    },
    /// No revision remains; no write occurred.
    Exhausted,
}

/// Structural corruption is never interpreted as a successful empty list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchSnapshotProblem {
    /// The singleton row is missing.
    Missing,
    /// An unexpected singleton identity or extra row exists.
    Cardinality,
    /// The stored body exceeds the shared payload budget.
    TooLarge,
}

pub(crate) fn read(connection: &Connection) -> Result<LaunchProfilesSnapshot, StoreError> {
    let cap = i64::try_from(LaunchPayload::MAX_BYTES)
        .map_err(|source| StoreError::LaunchRevision { source })?;
    // CASE prevents an oversized TEXT value from crossing the SQL boundary;
    // Rust never allocates that body merely to discover it exceeds the cap.
    let mut statement = connection.prepare(
        "SELECT id, revision, CASE WHEN length(CAST(profiles AS BLOB)) <= ?1
         THEN profiles ELSE NULL END FROM launch_profiles LIMIT 2",
    )?;
    let mut rows = statement.query([cap])?;
    let row = rows
        .next()?
        .ok_or(StoreError::LaunchSnapshot(LaunchSnapshotProblem::Missing))?;
    let id: i64 = row.get(0)?;
    if id != 1 {
        return Err(StoreError::LaunchSnapshot(
            LaunchSnapshotProblem::Cardinality,
        ));
    }
    let raw_revision: i64 = row.get(1)?;
    let revision =
        u32::try_from(raw_revision).map_err(|source| StoreError::LaunchRevision { source })?;
    let body: Option<String> = row.get(2)?;
    let body = body.ok_or(StoreError::LaunchSnapshot(LaunchSnapshotProblem::TooLarge))?;
    let profiles = serde_json::from_str(&body).map_err(StoreError::LaunchDecode)?;
    if rows.next()?.is_some() {
        return Err(StoreError::LaunchSnapshot(
            LaunchSnapshotProblem::Cardinality,
        ));
    }
    Ok(LaunchProfilesSnapshot::new(
        LaunchProfileRevision::new(revision),
        profiles,
    ))
}

pub(crate) fn replace(
    connection: &Connection,
    expected: LaunchProfileRevision,
    profiles: &LaunchProfiles,
) -> Result<LaunchProfilesChange, StoreError> {
    let current = read(connection)?;
    // Staleness takes precedence even when the payload happens to match.
    if current.revision() != expected {
        return Ok(LaunchProfilesChange::Stale {
            current: current.revision(),
        });
    }
    if current.profiles() == profiles {
        return Ok(LaunchProfilesChange::Unchanged(current));
    }
    let Some(revision) = current.revision().next() else {
        return Ok(LaunchProfilesChange::Exhausted);
    };
    let body = serde_json::to_string(profiles).map_err(StoreError::Encode)?;
    connection.execute(
        "UPDATE launch_profiles SET revision = ?1, profiles = ?2 WHERE id = 1",
        params![i64::from(revision.get()), body],
    )?;
    Ok(LaunchProfilesChange::Replaced(LaunchProfilesSnapshot::new(
        revision,
        profiles.clone(),
    )))
}

#[cfg(test)]
mod tests;
