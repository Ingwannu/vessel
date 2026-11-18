//! The usage ledger: atomic projection of reports, independent of log
//! pruning. No transcript reads or provider requests (ADR 0042).

mod cost;
#[cfg(test)]
mod cost_tests;
mod money;
#[cfg(test)]
mod precision_tests;
mod read;
#[cfg(test)]
mod tests;
mod totals;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::usage::{UsageSample, UsageSampleKind};
use vessel_proto::{AgentEvent, EventRecord, SessionEvent, SessionId, SessionSpec};

use crate::codec::{decode, encode, seq_to_sql, time_to_sql};
use crate::error::{Column, StoreError};

pub(crate) use cost::initialize as initialize_cost;
pub(crate) use read::query;
/// Maximum number of retained samples; limits query work and disk space.
const MAX_SAMPLES: u32 = 100_000;
const DAYS_KEPT: i64 = 90;

/// Called inside the event append's savepoint. A failed projection rolls
/// back its event too, so neither can claim a report the other lost.
pub(crate) fn observe(
    connection: &Connection,
    session: SessionId,
    record: &EventRecord,
) -> Result<(), StoreError> {
    let kind = match &record.event {
        SessionEvent::Agent {
            event: AgentEvent::Ready {
                harness_session, ..
            },
        }
        | SessionEvent::Rewound {
            harness_session, ..
        } => {
            cost::ready(connection, session, harness_session)?;
            return Ok(());
        }
        SessionEvent::Status {
            status: vessel_proto::SessionStatus::Starting,
        }
        | SessionEvent::Parked { .. } => {
            cost::starting(connection, session)?;
            return Ok(());
        }
        // A reload (TRACKER 803) is a wake's restart on the same
        // conversation: its Ready is consumed, its counter goes on.
        SessionEvent::Woken | SessionEvent::Reloaded { .. } => {
            cost::woken(connection, session)?;
            return Ok(());
        }
        SessionEvent::Forked { .. } => {
            cost::changed_profile(connection, session)?;
            return Ok(());
        }
        SessionEvent::Agent {
            event: AgentEvent::TurnFailed { .. },
        } => UsageSampleKind::Turn { usage: None },
        SessionEvent::Agent {
            event: AgentEvent::TurnEnded { usage, .. },
        } => UsageSampleKind::Turn { usage: *usage },
        SessionEvent::UsageChanged { usage } => {
            let Some(cost) = &usage.cost else {
                return Ok(());
            };
            let Some(kind) =
                cost::increment(connection, session, cost, record.at, usage.observation)?
            else {
                return Ok(());
            };
            kind
        }
        SessionEvent::Agent { .. }
        | SessionEvent::Terminal(_)
        | SessionEvent::ModelCall(_)
        | SessionEvent::Status { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::ProfileChanged { .. } => return Ok(()),
    };
    let (spec, profile): (String, Option<String>) = connection.query_row(
        "SELECT spec, profile FROM sessions WHERE id = ?1",
        [session.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let spec: SessionSpec = decode(session, Column::Spec, &spec)?;
    // A profile change and a sample can share a batch. Read preceding
    // changes from the row updated below by note_profile, not batch state.
    let sample = UsageSample {
        seq: record.seq,
        at: record.at,
        kind,
    };
    let _rows = connection.execute(
        "INSERT INTO usage_samples(session_id, seq, at, harness, profile, sample) VALUES (?1,?2,?3,?4,?5,?6)",
        params![session.to_string(), seq_to_sql(record.seq)?, time_to_sql(record.at)?, encode(&spec.harness)?, profile.unwrap_or_default(), encode(&sample)?],
    )?;
    Ok(())
}

pub(crate) fn note_profile(
    connection: &Connection,
    session: SessionId,
    record: &EventRecord,
) -> Result<(), StoreError> {
    if let SessionEvent::ProfileChanged { to, .. } = &record.event {
        cost::changed_profile(connection, session)?;
        let _rows = connection.execute(
            "UPDATE sessions SET profile = ?2 WHERE id = ?1",
            params![
                session.to_string(),
                to.as_ref().map(vessel_proto::ProfileName::as_str)
            ],
        )?;
    }
    Ok(())
}

/// Age and row caps are enforced once per event batch, never per token.
pub(crate) fn prune(connection: &Connection, now: DateTime<Utc>) -> Result<(), StoreError> {
    prune_to(connection, now, MAX_SAMPLES)
}

fn prune_to(connection: &Connection, now: DateTime<Utc>, maximum: u32) -> Result<(), StoreError> {
    let cutoff = time_to_sql(before(now, DAYS_KEPT))?;
    // Triggers maintain the count, including cascading session deletion.
    // Ordinary batches avoid walking the 100,000-row cap index at all.
    let count: i64 =
        connection.query_row("SELECT sample_count FROM usage_meta", [], |row| row.get(0))?;
    if count > i64::from(maximum) {
        let through: Option<i64> = connection.query_row(
            "SELECT at FROM usage_samples ORDER BY at DESC, session_id DESC, seq DESC LIMIT 1 OFFSET ?1", [maximum], |row| row.get(0),
        ).optional()?;
        if let Some(at) = through {
            let _rows = connection.execute(
                "UPDATE usage_meta SET pruned_through = MAX(COALESCE(pruned_through, ?1), ?1)",
                [at],
            )?;
            // Timestamp ties must not delete every sample of a burst.
            let _rows = connection.execute(
                "DELETE FROM usage_samples WHERE (session_id, seq) IN (
                    SELECT session_id, seq FROM usage_samples
                    ORDER BY at DESC, session_id DESC, seq DESC LIMIT -1 OFFSET ?1)",
                [maximum],
            )?;
        }
    }
    let _rows = connection.execute("DELETE FROM usage_samples WHERE at < ?1", [cutoff])?;
    Ok(())
}

use rusqlite::OptionalExtension as _;

pub(super) fn before(now: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    now.checked_sub_signed(chrono::TimeDelta::days(days))
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}
