//! Schedules and their runs: decisions shared by every renderer, without I/O.
//!
//! [Decision Log]
//! - Purpose: reproduce Paseo's active/ended list and editable schedule rows.
//! - Constraint: the page has no schedule screen. Paseo schedule-format tests
//!   provide the formatting cases; the node already
//!   supplies authoritative states, typed cadence and bounded run history.
//! - Alternatives: infer expiry or missing targets from a client clock/cache;
//!   duplicate the cron parser; reuse the node's protocol values.
//! - Choice: use protocol values and exhaustive decisions, with portable JSON
//!   cases for a future page. Never infer target deletion from a partial fleet.
//! - Why: local caches and clocks cannot overrule a node's runnable state.
//! - Cost: a newer unknown variant is readable but cannot be edited here.

pub mod authority;
pub mod cadence;
mod metadata;
pub use metadata::Calendar;
pub mod presentation;
pub mod time;
pub use crate::badge::Variant;
pub mod description;
pub mod editor;
pub mod load;
pub mod locale;
pub mod page;
pub mod target;

use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    EndReason, ScheduleDefinition, ScheduleInfo, ScheduleState, ScheduleTarget,
};

/// Paseo's two schedule filters, plus unknown states retained for inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Active and paused schedules.
    Active,
    /// Schedules the node ended.
    Ended,
    /// A newer node's state; kept separate from ended schedules.
    Unknown,
}

/// A schedule's available action; the screen only draws these actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Stop automatic runs.
    Pause,
    /// Let a paused schedule run again.
    Resume,
    /// Ask for a run outside its cadence (including while paused).
    RunNow,
    /// Delete its definition and history; the sessions remain.
    Delete,
}

/// Existing-session targets are heartbeats in the UI. The node tracks
/// ownership separately for authorization and lifetime.
#[must_use]
pub const fn is_heartbeat(info: &ScheduleInfo) -> bool {
    matches!(&info.definition.target, ScheduleTarget::Session { .. })
}

/// The node's state, with its ended reason retained for a precise badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Automatically runnable.
    Active,
    /// Waiting for a person to resume.
    Paused,
    /// No further runs; why.
    Ended(EndReason),
    /// Readable but unknown to this client.
    Unknown,
}

impl Status {
    /// Which shared status badge variant draws this state.
    #[must_use]
    pub const fn badge(self) -> Variant {
        match self {
            Self::Active => Variant::Success,
            Self::Paused | Self::Unknown => Variant::Muted,
            Self::Ended(reason) => match reason {
                EndReason::MaxRuns | EndReason::Expired | EndReason::Unknown => Variant::Muted,
                EndReason::CreatorGone | EndReason::TargetGone => Variant::Error,
            },
        }
    }
    /// Derive solely from the node, never from the session cache or local time.
    #[must_use]
    pub const fn of(state: &ScheduleState) -> Self {
        match state {
            ScheduleState::Active { .. } => Self::Active,
            ScheduleState::Paused { .. } => Self::Paused,
            ScheduleState::Ended { reason, .. } => Self::Ended(*reason),
            ScheduleState::Unknown => Self::Unknown,
        }
    }

    /// Which filter contains this row.
    #[must_use]
    pub const fn bucket(self) -> Bucket {
        match self {
            Self::Active | Self::Paused => Bucket::Active,
            Self::Ended(_) => Bucket::Ended,
            Self::Unknown => Bucket::Unknown,
        }
    }
}

/// A title borrowed intact: named schedule, new session's title, prompt's first
/// nonempty line (Paseo's precedence). The row truncates the title to one line.
#[must_use]
pub fn title(definition: &ScheduleDefinition) -> &str {
    if let Some(name) = &definition.name {
        return name.as_str();
    }
    if let ScheduleTarget::NewSession { spec, .. } = &definition.target
        && let Some(title) = &spec.title
    {
        return title.as_str();
    }
    // PromptText's constructor guarantees a nonblank prompt. A fallback keeps
    // this function total if that contract ever becomes more permissive.
    definition
        .prompt
        .as_str()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
}

/// Node-reachable row operations. A heartbeat keeps its owner's cadence.
#[must_use]
pub fn actions(schedule: &ScheduleInfo, authority: &authority::Authority) -> Vec<Action> {
    if !authority.allows_actions() {
        return Vec::new();
    }
    if is_heartbeat(schedule) {
        return vec![Action::Delete];
    }
    match Status::of(&schedule.state) {
        Status::Active => vec![Action::Pause, Action::RunNow, Action::Delete],
        Status::Paused => vec![Action::Resume, Action::RunNow, Action::Delete],
        Status::Ended(_) | Status::Unknown => vec![Action::Delete],
    }
}

/// Next due instant only for an active schedule. The renderer never presents a
/// paused schedule's old due time as a future promise.
#[must_use]
pub const fn next_due(schedule: &ScheduleInfo) -> Option<DateTime<Utc>> {
    match schedule.state {
        ScheduleState::Active { next_due } => Some(next_due),
        ScheduleState::Paused { .. } | ScheduleState::Ended { .. } | ScheduleState::Unknown => None,
    }
}

/// One machine's visible rows, newest first, ties resolved by schedule id so
/// an arbitrary node response order cannot move keyboard position.
#[must_use]
pub fn rows(schedules: &[ScheduleInfo], bucket: Bucket) -> Vec<&ScheduleInfo> {
    let mut rows: Vec<_> = schedules
        .iter()
        .filter(|info| Status::of(&info.state).bucket() == bucket)
        .collect();
    rows.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    rows
}

#[cfg(test)]
mod tests;
