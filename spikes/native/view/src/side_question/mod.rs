//! Side-question decisions from GOAT's `sideChat.logic`: a command in the
//! composer, the newest side of one node's parent, and its header state.
//!
//! [Decision Log]
//! - Purpose: a side must be visibly separate while its parent keeps working.
//! - Constraint: goals and full provider-history forks are absent on this base.
//! - Choice: existing node sessions and timeline; pure values here, transport
//!   in link, drawing in the side panel. Never infer a missing goal or resume.
//! - Cost: context is partial and an ended side needs explicit resume.

use vessel_proto::automation::{InvalidAutomationText, PromptText};
use vessel_proto::{Activity, Conversation, SessionId, SessionInfo, SessionMode, SessionStatus};

mod request;
pub use request::{Exhausted, Failure, Ticket, Tickets};
mod submission;
mod target;
pub use submission::{Submission, SubmitProblem, submission};
pub use target::{BookEffect, Target};

#[cfg(test)]
mod tests;

/// The composer's `/side` action, before it reaches the parent prompt path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideCommand {
    /// Open the panel with an empty question field.
    Open,
    /// Open the panel and ask once its side session is ready.
    Ask(PromptText),
}

/// Parse GOAT's complete `/side` command, case-insensitive, with multiline
/// questions. Prose or a longer command is ordinary composer text.
///
/// # Errors
/// The question violates the node's existing bounded plain-text type.
pub fn command(text: &str) -> Option<Result<SideCommand, InvalidAutomationText>> {
    let text = text.trim_matches(crate::timeline::is_space);
    if !text
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("/side"))
    {
        return None;
    }
    let tail = text.get(5..)?;
    if tail
        .chars()
        .next()
        .is_some_and(|character| !crate::timeline::is_space(character))
    {
        return None;
    }
    let question = tail.trim_matches(crate::timeline::is_space);
    Some(if question.is_empty() {
        Ok(SideCommand::Open)
    } else {
        question.parse().map(SideCommand::Ask)
    })
}

/// Whether this parent can have a side. Scope checks remain the node's;
/// a nested side or terminal must never offer an action that will refuse.
#[must_use]
pub fn can_open(parent: Option<&SessionInfo>) -> bool {
    parent.is_some_and(|parent| {
        parent.spec.side_question_of.is_none()
            && matches!(parent.spec.mode, SessionMode::Structured { .. })
    })
}

/// Newest unarchived side among sessions from this one node. Ended sides
/// remain discoverable; opening the panel must not silently resume them.
#[must_use]
pub fn newest(sessions: &[SessionInfo], parent: SessionId) -> Option<&SessionInfo> {
    sessions
        .iter()
        .filter(|session| {
            session.spec.side_question_of == Some(parent) && session.marks.archived_at.is_none()
        })
        .max_by_key(|session| (session.created_at, session.id))
}

/// The parent's short header status. A goal-paused state is not fabricated
/// because the node does not have GOAT's goal projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentStatus {
    /// A tool needs permission, ahead of every other state.
    NeedsApproval,
    /// The agent needs a question answered.
    NeedsInput,
    /// Its process is starting.
    Starting,
    /// A turn is running.
    Working,
    /// Its latest turn was interrupted.
    Interrupted,
    /// Its process or turn failed.
    Failed,
    /// Between turns or finished normally.
    Idle,
}

/// The status dot's semantic color, chosen before drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentTone {
    /// Starting or working.
    Active,
    /// Waiting for the person.
    Attention,
    /// Resting or interrupted.
    Idle,
    /// Failure.
    Error,
}

impl ParentStatus {
    /// GOAT's status-to-tone mapping.
    #[must_use]
    pub const fn tone(self) -> ParentTone {
        match self {
            Self::NeedsApproval | Self::NeedsInput => ParentTone::Attention,
            Self::Starting | Self::Working => ParentTone::Active,
            Self::Interrupted | Self::Idle => ParentTone::Idle,
            Self::Failed => ParentTone::Error,
        }
    }
}

/// Approval, then question, then live work, then last outcome: GOAT's
/// precedence. Conversation is read separately from the partial transcript.
#[must_use]
pub fn parent_status(parent: &SessionInfo, conversation: &Conversation) -> ParentStatus {
    if !conversation.permissions.is_empty() {
        return ParentStatus::NeedsApproval;
    }
    if !conversation.questions.is_empty() {
        return ParentStatus::NeedsInput;
    }
    if parent
        .board
        .as_ref()
        .is_some_and(|board| board.display_status == vessel_proto::DisplayStatus::Interrupted)
    {
        return ParentStatus::Interrupted;
    }
    match parent.status {
        SessionStatus::Starting => ParentStatus::Starting,
        SessionStatus::Running => match parent.board.as_ref().map(|board| board.activity) {
            Some(Activity::Active) => ParentStatus::Working,
            Some(Activity::WaitingInput) => ParentStatus::NeedsInput,
            Some(Activity::Idle | Activity::Exited) | None => ParentStatus::Idle,
        },
        SessionStatus::Interrupted => ParentStatus::Interrupted,
        SessionStatus::Exited { .. } => ParentStatus::Idle,
        SessionStatus::Failed { .. } => ParentStatus::Failed,
    }
}
