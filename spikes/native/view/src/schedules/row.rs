//! One row of the list: kind, badge, menu and the parts a catalog renders.
//!
//! The badge is the node's state. A missing session does not change it: the
//! node records target-gone itself (ADR 0072).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    Cadence, EndReason, PromptText, RunOutcome, ScheduleInfo, ScheduleName, ScheduleState,
    ScheduleTarget,
};
use vessel_proto::{Creator, ScheduleId, SessionTitle};

use super::{SchedulePeer, ScheduleReach, SessionDirectory};

/// Heartbeat, or a schedule a person owns the lifetime of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleKind {
    /// `defined_by` is an agent and the target is that agent's session.
    /// Deleted when that session ends.
    Heartbeat,
    /// Anything else the node keeps, including a person's session target.
    Schedule,
}

/// The badge, one per node state. `Unknown` is kept, not dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleBadge {
    /// `ScheduleState::Active`.
    Active,
    /// `ScheduleState::Paused`.
    Paused,
    /// Ended because it reached its cap.
    Finished,
    /// Ended because the next due time was past `ends_at`.
    Expired,
    /// Ended because the target is gone.
    TargetGone,
    /// Ended because the defining device was revoked.
    CreatorGone,
    /// An ended reason or a state this app does not know.
    Unknown,
}

/// The Active / Ended filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleBucket {
    /// Active or paused.
    Runnable,
    /// Every ended reason, and an unknown state.
    Ended,
}

/// A menu control the row may send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Open the editor. Absent when a save would be refused.
    Edit,
    /// `schedule.pause`.
    Pause,
    /// `schedule.resume`.
    Resume,
    /// `schedule.run`.
    RunNow,
    /// `schedule.delete`. Does not change `defined_by`.
    Delete,
}

/// Where the row's press goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowOpen {
    /// A person's schedule on a direct peer: the editor can save.
    Editor,
    /// A heartbeat, an agent-owned schedule, or a forwarded node: no save.
    Detail,
}

/// Why run now is hidden besides the peer and the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunHold {
    /// No run of this schedule is waiting on a reply, and the last outcome
    /// is not `Making`.
    Clear,
    /// The last known outcome is `Making`, or a run request is outstanding.
    Held,
}

/// What the title is taken from, in Paseo's order: name, session title, the
/// first prompt line, then a fallback the catalog words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleSource {
    /// `ScheduleName`.
    Name(ScheduleName),
    /// `SessionSpec.title` of a new-session target.
    SessionTitle(SessionTitle),
    /// The first non-empty line of the prompt.
    PromptLine(PromptLine),
    /// The prompt had no line to show. `PromptText` refuses a blank value,
    /// so this is a newer node's odd record.
    Fallback,
}

/// One line of a prompt, trimmed, carried so the row does not hand the
/// screen a bare `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptLine(Arc<str>);

impl PromptLine {
    /// The line as trimmed.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Row meta, in Paseo's order. The instants are the node's. A relative
/// phrase is the catalog's, not this type's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaPart {
    /// The cadence as stored.
    Cadence(Cadence),
    /// When the schedule was created.
    Created(DateTime<Utc>),
    /// When it last ran. `None` is "never run".
    LastRun(Option<DateTime<Utc>>),
    /// The next due time. Only while `Active`.
    NextDue(DateTime<Utc>),
}

/// A row the list has decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleRow {
    /// The node it was listed from.
    pub node: vessel_proto::NodeId,
    /// The node's record.
    pub info: ScheduleInfo,
    /// Heartbeat or schedule.
    pub kind: ScheduleKind,
    /// The node's state, as a badge.
    pub badge: ScheduleBadge,
    /// Which filter it belongs to.
    pub bucket: ScheduleBucket,
    /// Title parts, not a sentence.
    pub title: TitleSource,
    /// Meta parts, in order.
    pub meta: Vec<MetaPart>,
    /// Menu controls that may be sent.
    pub actions: Vec<MenuAction>,
    /// Editor, or a detail with no save.
    pub open: RowOpen,
    /// Whether the session directory had loaded. The badge does not read it.
    pub directory: SessionDirectory,
}

/// Heartbeat only when the agent that wrote the schedule is the session it
/// prompts. A person's session target stays a schedule: it is not deleted
/// when a session ends.
#[must_use]
pub fn kind(info: &ScheduleInfo) -> ScheduleKind {
    match (&info.defined_by, &info.definition.target) {
        (Creator::Agent { session: owner, .. }, ScheduleTarget::Session { session })
            if owner == session =>
        {
            ScheduleKind::Heartbeat
        }
        (
            Creator::Agent { .. },
            ScheduleTarget::Session { .. }
            | ScheduleTarget::NewSession { .. }
            | ScheduleTarget::Unknown,
        )
        | (
            Creator::Owner | Creator::Device { .. },
            ScheduleTarget::NewSession { .. }
            | ScheduleTarget::Session { .. }
            | ScheduleTarget::Unknown,
        ) => ScheduleKind::Schedule,
    }
}

/// The badge is the node's state, including creator-gone and unknown.
#[must_use]
pub fn badge(state: &ScheduleState) -> ScheduleBadge {
    match state {
        ScheduleState::Active { .. } => ScheduleBadge::Active,
        ScheduleState::Paused { .. } => ScheduleBadge::Paused,
        ScheduleState::Ended { reason, .. } => match reason {
            EndReason::MaxRuns => ScheduleBadge::Finished,
            EndReason::Expired => ScheduleBadge::Expired,
            EndReason::TargetGone => ScheduleBadge::TargetGone,
            EndReason::CreatorGone => ScheduleBadge::CreatorGone,
            EndReason::Unknown => ScheduleBadge::Unknown,
        },
        ScheduleState::Unknown => ScheduleBadge::Unknown,
    }
}

/// Active and paused are runnable. Ended and unknown are the Ended filter.
#[must_use]
pub fn bucket(state: &ScheduleState) -> ScheduleBucket {
    match state {
        ScheduleState::Active { .. } | ScheduleState::Paused { .. } => ScheduleBucket::Runnable,
        ScheduleState::Ended { .. } | ScheduleState::Unknown => ScheduleBucket::Ended,
    }
}

/// Name, else the new session's title, else the first prompt line.
#[must_use]
pub fn title(info: &ScheduleInfo) -> TitleSource {
    if let Some(name) = &info.definition.name {
        return TitleSource::Name(name.clone());
    }
    if let ScheduleTarget::NewSession { spec, .. } = &info.definition.target
        && let Some(session_title) = &spec.title
    {
        return TitleSource::SessionTitle(session_title.clone());
    }
    first_line(&info.definition.prompt)
}

/// Menu for this record. A heartbeat is delete only. Run now needs a direct
/// peer and a clear hold. Edit needs a direct peer and a running person's
/// schedule, because a save changes `defined_by`.
#[must_use]
pub fn actions(info: &ScheduleInfo, peer: &SchedulePeer, hold: RunHold) -> Vec<MenuAction> {
    if kind(info) == ScheduleKind::Heartbeat {
        return vec![MenuAction::Delete];
    }
    let SchedulePeer::Ready(ready) = peer else {
        return vec![MenuAction::Delete];
    };
    let mut actions = Vec::new();
    if ready.reach == ScheduleReach::Direct && can_edit(info) {
        actions.push(MenuAction::Edit);
    }
    match &info.state {
        ScheduleState::Active { .. } => actions.push(MenuAction::Pause),
        ScheduleState::Paused { .. } => actions.push(MenuAction::Resume),
        ScheduleState::Ended { .. } | ScheduleState::Unknown => {}
    }
    if ready.reach == ScheduleReach::Direct && can_run(info, hold) {
        actions.push(MenuAction::RunNow);
    }
    actions.push(MenuAction::Delete);
    actions
}

/// Editor only when edit is offered. Every other row opens the detail.
#[must_use]
pub fn open_of(actions: &[MenuAction]) -> RowOpen {
    if actions.contains(&MenuAction::Edit) {
        RowOpen::Editor
    } else {
        RowOpen::Detail
    }
}

/// Meta in order. The next due time is present only while the schedule is
/// active.
#[must_use]
pub fn meta(info: &ScheduleInfo) -> Vec<MetaPart> {
    let mut parts = vec![
        MetaPart::Cadence(info.definition.cadence.clone()),
        MetaPart::Created(info.created_at),
        MetaPart::LastRun(info.last_run.as_ref().map(|run| run.at)),
    ];
    if let ScheduleState::Active { next_due } = info.state {
        parts.push(MetaPart::NextDue(next_due));
    }
    parts
}

/// Run now is held while a run is being made or a request is unanswered.
#[must_use]
pub fn hold_of(info: &ScheduleInfo, outstanding: &[ScheduleId]) -> RunHold {
    if outstanding.contains(&info.id) || last_is_making(info) {
        RunHold::Held
    } else {
        RunHold::Clear
    }
}

fn can_edit(info: &ScheduleInfo) -> bool {
    matches!(info.defined_by, Creator::Owner | Creator::Device { .. })
        && matches!(
            info.state,
            ScheduleState::Active { .. } | ScheduleState::Paused { .. }
        )
        && definition_is_known(info)
}

fn can_run(info: &ScheduleInfo, hold: RunHold) -> bool {
    hold == RunHold::Clear
        && matches!(
            info.state,
            ScheduleState::Active { .. } | ScheduleState::Paused { .. }
        )
}

fn definition_is_known(info: &ScheduleInfo) -> bool {
    !matches!(info.definition.cadence, Cadence::Unknown)
        && !matches!(info.definition.target, ScheduleTarget::Unknown)
        && !matches!(
            info.definition.catch_up,
            vessel_proto::automation::CatchUp::Unknown
        )
        && match &info.definition.target {
            ScheduleTarget::NewSession { after, .. } => {
                !matches!(after, vessel_proto::automation::AfterRun::Unknown)
            }
            ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => true,
        }
}

fn last_is_making(info: &ScheduleInfo) -> bool {
    match info.last_run.as_ref().map(|run| &run.outcome) {
        Some(RunOutcome::Making) => true,
        Some(
            RunOutcome::Started { .. }
            | RunOutcome::Waiting { .. }
            | RunOutcome::Finished { .. }
            | RunOutcome::Prompted { .. }
            | RunOutcome::Skipped { .. }
            | RunOutcome::Failed { .. }
            | RunOutcome::Interrupted
            | RunOutcome::Unknown,
        )
        | None => false,
    }
}

fn first_line(prompt: &PromptText) -> TitleSource {
    for line in prompt.as_str().lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return TitleSource::PromptLine(PromptLine(Arc::from(trimmed)));
        }
    }
    TitleSource::Fallback
}
