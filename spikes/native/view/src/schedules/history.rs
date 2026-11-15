//! History decisions. The node retains at most fifty runs; the original typed
//! reason remains beside the display state so failure details are never lost.

pub mod reader;

use vessel_proto::automation::{RunEnd, RunOutcome};
use vessel_proto::conversation::Delivered;
use vessel_proto::{SessionId, SessionStatus, StopReason};

/// A run's concise state; detailed reasons remain on the original run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The node recorded its intent before making the run.
    Making,
    /// Its first turn is under way.
    Running,
    /// A person must answer a permission.
    Waiting,
    /// Its first turn finished normally.
    Finished,
    /// A prompt started in the existing session.
    Prompted,
    /// A newer or unexpected caller steered into the active turn.
    Steered,
    /// The prompt waits in the existing session's durable queue.
    Queued,
    /// No prompt was sent; the typed skip reason explains why.
    Skipped,
    /// The call, turn or session failed.
    Failed,
    /// A person cancelled the turn.
    Cancelled,
    /// The model reached a token or request limit.
    Limited,
    /// The model refused the request.
    Refused,
    /// The session ended before the turn completed.
    SessionEnded,
    /// The node stopped during creation or the session was interrupted.
    Interrupted,
    /// A newer node's result.
    Unknown,
}

/// Classify every outcome without treating a failed finished turn as success.
#[must_use]
pub const fn outcome(value: &RunOutcome) -> Outcome {
    match value {
        RunOutcome::Making => Outcome::Making,
        RunOutcome::Started { .. } => Outcome::Running,
        RunOutcome::Waiting { .. } => Outcome::Waiting,
        RunOutcome::Finished { end, .. } => ended(end),
        RunOutcome::Prompted { delivered, .. } => match delivered {
            Delivered::Started => Outcome::Prompted,
            Delivered::Steered => Outcome::Steered,
            Delivered::Queued { .. } => Outcome::Queued,
        },
        RunOutcome::Skipped { .. } => Outcome::Skipped,
        RunOutcome::Failed { .. } => Outcome::Failed,
        RunOutcome::Interrupted => Outcome::Interrupted,
        RunOutcome::Unknown => Outcome::Unknown,
    }
}

const fn ended(end: &RunEnd) -> Outcome {
    match end {
        RunEnd::Turn { reason } => match reason {
            StopReason::EndTurn => Outcome::Finished,
            StopReason::MaxTokens | StopReason::MaxTurnRequests => Outcome::Limited,
            StopReason::Refusal => Outcome::Refused,
            StopReason::Cancelled => Outcome::Cancelled,
        },
        RunEnd::Failed { .. } => Outcome::Failed,
        RunEnd::Session { status } => match status {
            SessionStatus::Starting | SessionStatus::Running => Outcome::Unknown,
            SessionStatus::Exited {
                code: None | Some(0),
            } => Outcome::SessionEnded,
            SessionStatus::Exited { code: Some(_) } | SessionStatus::Failed { .. } => {
                Outcome::Failed
            }
            SessionStatus::Interrupted => Outcome::Interrupted,
        },
        RunEnd::Unknown => Outcome::Unknown,
    }
}

/// Session to open, including a failed creation that got as far as a session.
/// No link is drawn for a run that never made one.
#[must_use]
pub const fn session(value: &RunOutcome) -> Option<SessionId> {
    match value {
        RunOutcome::Started { session }
        | RunOutcome::Waiting { session }
        | RunOutcome::Finished { session, .. }
        | RunOutcome::Prompted { session, .. } => Some(*session),
        RunOutcome::Failed { session, .. } => *session,
        RunOutcome::Making
        | RunOutcome::Skipped { .. }
        | RunOutcome::Interrupted
        | RunOutcome::Unknown => None,
    }
}
