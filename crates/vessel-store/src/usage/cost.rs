//! ACP cost is cumulative for a native session, not a process. Starting
//! or replaying a process must never spend that same amount twice.

use crate::codec::{decode, encode};
use crate::error::{Column, StoreError};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::{Deserialize, Serialize};
use vessel_proto::conversation::{Cost, UsageObservation};
use vessel_proto::usage::UsageSampleKind;
use vessel_proto::{HarnessSessionId, SessionId, SessionInfo, SessionMode};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
enum Origin {
    Fresh,
    #[default]
    Existing,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    #[default]
    Starting,
    Ready,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Baseline {
    cost: Cost,
    at: DateTime<Utc>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    identity: Option<HarnessSessionId>,
    origin: Origin,
    phase: Phase,
    baselines: Vec<Baseline>,
}

pub(crate) fn initialize(connection: &Connection, info: &SessionInfo) -> Result<(), StoreError> {
    let started: i64 =
        connection.query_row("SELECT started_at FROM usage_meta", [], |row| row.get(0))?;
    let new_origin = match &info.origin {
        None | Some(vessel_proto::SessionOrigin::Remote { .. }) => true,
        Some(vessel_proto::SessionOrigin::Imported { .. }) => false,
    };
    let origin = if crate::codec::time_to_sql(info.created_at)? >= started
        && matches!(info.spec.mode, SessionMode::Structured { resume: None, .. })
        && new_origin
    {
        Origin::Fresh
    } else {
        Origin::Existing
    };
    put(
        connection,
        info.id,
        &State {
            origin,
            ..State::default()
        },
    )
}
fn get(connection: &Connection, session: SessionId) -> Result<State, StoreError> {
    let text: Option<Option<String>> = connection
        .query_row(
            "SELECT usage_cost_baseline FROM sessions WHERE id = ?1",
            [session.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    text.flatten()
        .map(|text| decode(session, Column::Usage, &text))
        .transpose()
        .map(Option::unwrap_or_default)
}

fn put(connection: &Connection, session: SessionId, state: &State) -> Result<(), StoreError> {
    let _rows = connection.execute(
        "UPDATE sessions SET usage_cost_baseline = ?2 WHERE id = ?1",
        params![session.to_string(), encode(state)?],
    )?;
    Ok(())
}
pub(super) fn starting(connection: &Connection, session: SessionId) -> Result<(), StoreError> {
    let mut state = get(connection, session)?;
    state.phase = Phase::Starting;
    put(connection, session, &state)
}
/// A wake's Ready is consumed by the session's replay filter. Woken is
/// its durable completion marker; it must not reset a cumulative counter.
pub(super) fn woken(connection: &Connection, session: SessionId) -> Result<(), StoreError> {
    let mut state = get(connection, session)?;
    state.phase = Phase::Ready;
    put(connection, session, &state)
}
pub(super) fn ready(
    connection: &Connection,
    session: SessionId,
    identity: &HarnessSessionId,
) -> Result<(), StoreError> {
    let mut state = get(connection, session)?;
    if state
        .identity
        .as_ref()
        .is_some_and(|previous| previous != identity)
    {
        state.baselines.clear();
        state.origin = Origin::Existing;
    }
    state.identity = Some(identity.clone());
    state.phase = Phase::Ready;
    put(connection, session, &state)
}
pub(super) fn changed_profile(
    connection: &Connection,
    session: SessionId,
) -> Result<(), StoreError> {
    let mut state = get(connection, session)?;
    // The next cumulative observation cannot be attributed wholly to a
    // profile that only just took over. Its total remains visible as such.
    state.baselines.clear();
    state.origin = Origin::Existing;
    put(connection, session, &state)
}
pub(crate) fn current(
    connection: &Connection,
    session: SessionId,
) -> Result<Vec<Cost>, StoreError> {
    let tracked: Vec<_> = get(connection, session)?
        .baselines
        .into_iter()
        .map(|baseline| baseline.cost)
        .collect();
    if !tracked.is_empty() {
        return Ok(tracked);
    }
    // Older ended sessions already have a typed last report. Showing it
    // is not a historical spend backfill: no ledger increment is created.
    let text: Option<Option<String>> = connection
        .query_row(
            "SELECT usage FROM sessions WHERE id = ?1",
            [session.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let usage: Option<vessel_proto::ContextUsage> = text
        .flatten()
        .map(|text| decode(session, Column::Usage, &text))
        .transpose()?;
    Ok(usage
        .and_then(|usage| usage.cost)
        .filter(|cost| cost.amount.get() >= 0.0)
        .into_iter()
        .collect())
}

pub(super) fn increment(
    connection: &Connection,
    session: SessionId,
    current: &Cost,
    at: DateTime<Utc>,
    observation: UsageObservation,
) -> Result<Option<UsageSampleKind>, StoreError> {
    if current.amount.get() < 0.0 {
        return Ok(None);
    }
    let mut state = get(connection, session)?;
    let phase = match observation {
        UsageObservation::Current => state.phase,
        UsageObservation::Replayed => Phase::Starting,
    };
    let previous = state
        .baselines
        .iter_mut()
        .find(|value| value.cost.unit == current.unit);
    let result = if let Some(previous) = previous {
        let result = change(previous, current, at, phase);
        *previous = Baseline {
            cost: current.clone(),
            at,
        };
        result
    } else {
        if state.baselines.len() >= 32 {
            return Ok(Some(UsageSampleKind::CostUnavailable));
        }
        state.baselines.push(Baseline {
            cost: current.clone(),
            at,
        });
        match (state.origin, phase) {
            (Origin::Fresh, Phase::Ready)
                if super::money::Amount::reported(current.amount).is_none() =>
            {
                Some(UsageSampleKind::CostUnavailable)
            }
            (Origin::Fresh, Phase::Ready) => {
                let created: i64 = connection.query_row(
                    "SELECT created_at FROM sessions WHERE id = ?1",
                    [session.to_string()],
                    |row| row.get(0),
                )?;
                Some(UsageSampleKind::Cost {
                    cost: current.clone(),
                    since: crate::codec::time_from_sql(created),
                })
            }
            (Origin::Existing, Phase::Ready | Phase::Starting)
            | (Origin::Fresh, Phase::Starting) => Some(UsageSampleKind::CostBaseline {
                cost: current.clone(),
            }),
        }
    };
    put(connection, session, &state)?;
    Ok(result)
}
fn change(
    previous: &Baseline,
    current: &Cost,
    at: DateTime<Utc>,
    phase: Phase,
) -> Option<UsageSampleKind> {
    if previous.cost.amount == current.amount {
        return None;
    }
    if current.amount < previous.cost.amount || phase == Phase::Starting || previous.at > at {
        return Some(UsageSampleKind::CostBaseline {
            cost: current.clone(),
        });
    }
    let amount = super::money::Amount::reported(current.amount)
        .zip(super::money::Amount::reported(previous.cost.amount))
        .and_then(|(current, previous)| current.difference(&previous))
        .and_then(super::money::Amount::wire);
    Some(match amount {
        Some(amount) => UsageSampleKind::Cost {
            cost: Cost {
                amount,
                unit: current.unit.clone(),
            },
            since: previous.at,
        },
        None => UsageSampleKind::CostUnavailable,
    })
}
