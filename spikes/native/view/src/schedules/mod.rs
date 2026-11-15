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
pub mod clock;
pub mod metadata;
pub mod presentation;
pub mod time;
pub use crate::badge::Variant;
pub mod description;
pub mod editor;
pub mod history;
pub mod load;
pub mod locale;
pub mod page;
pub mod target;

use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EndReason, ScheduleDefinition, ScheduleInfo, ScheduleState,
    ScheduleTarget,
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
    /// Change its definition, preserving the read revision.
    Edit,
    /// Stop automatic runs.
    Pause,
    /// Let a paused schedule run again.
    Resume,
    /// Ask for a run outside its cadence (including while paused).
    RunNow,
    /// Delete its definition and history; the sessions remain.
    Delete,
}

/// Product meaning of a schedule target, for row and editor titles/actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Product {
    /// Starts a new chat on each run.
    Schedule,
    /// Sends a recurring prompt into an existing chat's queue.
    RecurringPrompt,
    /// Target format from a newer machine.
    Unknown,
}

/// Classify a target without a raw wire tag becoming the feature's name.
#[must_use]
pub const fn product(target: &ScheduleTarget) -> Product {
    match target {
        ScheduleTarget::NewSession { .. } => Product::Schedule,
        ScheduleTarget::Session { .. } => Product::RecurringPrompt,
        ScheduleTarget::Unknown => Product::Unknown,
    }
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

/// Whether this client understands every replaceable part of a definition.
/// Unknown values must survive read-only inspection rather than being lost on save.
#[must_use]
pub const fn editable(definition: &ScheduleDefinition) -> bool {
    let cadence = match &definition.cadence {
        Cadence::Every { .. } | Cadence::Cron { .. } => true,
        Cadence::Unknown => false,
    };
    let catch_up = match definition.catch_up {
        CatchUp::Once | CatchUp::Skip => true,
        CatchUp::Unknown => false,
    };
    let target = match &definition.target {
        ScheduleTarget::NewSession { after, .. } => match after {
            AfterRun::Archive | AfterRun::Keep => true,
            AfterRun::Unknown => false,
        },
        ScheduleTarget::Session { .. } => true,
        ScheduleTarget::Unknown => false,
    };
    cadence && catch_up && target
}

/// Wired operations for a row. The menu separately applies Paseo's order and
/// heartbeat visibility. Node semantics additionally allow run-now, pause and
/// resume for an existing-session target (ADR 0033).
#[must_use]
pub fn actions(schedule: &ScheduleInfo, authority: &authority::Authority) -> Vec<Action> {
    if !authority.allows_actions() {
        return Vec::new();
    }
    let cadence_action = match Status::of(&schedule.state) {
        Status::Active => Action::Pause,
        Status::Paused => Action::Resume,
        Status::Ended(_) | Status::Unknown => return vec![Action::Delete],
    };
    let mut actions = Vec::with_capacity(4);
    if editable(&schedule.definition) {
        actions.push(Action::Edit);
    }
    actions.push(cadence_action);
    actions.push(Action::RunNow);
    actions.push(Action::Delete);
    actions
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
