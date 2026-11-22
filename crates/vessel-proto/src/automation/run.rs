//! A schedule's runs (protocol 11, ADR 0033): what each one did, or why it
//! did nothing, typed.
//!
//! [Decision Log]
//! - Purpose: a person looking at a schedule must see whether it ran,
//!   which session it started or prompted, and, when it did not, why: a
//!   previous run still going, due times missed while the node was down,
//!   or the call it made refused.
//! - Choice: one record per run with its trigger and outcome. A refused
//!   call is kept as the `WireError` that call answered (its code typed,
//!   its message cut), not as a reason of our own: the run went through
//!   `session.create` or `session.prompt`, whose failures are the
//!   protocol's already. A new session's run is followed to the end of
//!   its first turn (`started` becomes `finished`), as agent-orchestrator
//!   and Paseo follow theirs.
//! - Kept: the newest 50 per schedule. agent-orchestrator and Paseo keep
//!   every run unbounded; GOAT keeps the last one.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent::StopReason;
use crate::conversation::Delivered;
use crate::error::{ErrorCode, WireError};
use crate::id::{ScheduleId, SessionId};
use crate::profile::Creator;
use crate::session::SessionStatus;

/// A schedule's run, counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u32", into = "u32")]
pub struct RunNumber(u32);

impl RunNumber {
    /// The first run.
    pub const FIRST: Self = Self(1);

    /// Run `number`; `None` for 0 (runs count from 1).
    #[must_use]
    pub const fn new(number: u32) -> Option<Self> {
        if number == 0 {
            None
        } else {
            Some(Self(number))
        }
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The run after this one; `None` past `u32::MAX` (a schedule running
    /// once a minute takes eight thousand years to get there).
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl TryFrom<u32> for RunNumber {
    type Error = crate::automation::OutOfRange;

    fn try_from(number: u32) -> Result<Self, Self::Error> {
        Self::new(number).ok_or(crate::automation::OutOfRange {
            what: "a run number",
            min: 1,
            max: u32::MAX,
        })
    }
}

impl From<RunNumber> for u32 {
    fn from(number: RunNumber) -> Self {
        number.0
    }
}

/// One run of a schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScheduleRun {
    /// Which run.
    pub number: RunNumber,
    /// The due time it ran for: the latest, after a catch-up; the moment
    /// asked, for a run now.
    pub due_at: DateTime<Utc>,
    /// When it ran.
    pub at: DateTime<Utc>,
    /// What made it run.
    pub trigger: RunTrigger,
    /// What it did.
    pub outcome: RunOutcome,
    /// Workspace cleanup is independent of the turn result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<RunArchive>,
}

/// The disposition of an isolated workspace after a scheduled turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RunArchive {
    /// The isolated workspace was archived.
    Archived,
    /// The workspace was archived and its managed backing was kept for safety.
    BackingKept,
    /// The scheduled session has accepted work.
    DeferredBusy,
    /// Another member, agent or terminal, still lives in the workspace.
    DeferredOtherMembers,
    /// Cleanup could not confirm all effects.
    Unconfirmed,
    /// A result described by a newer node.
    #[serde(other)]
    Unknown,
}

/// What made a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "trigger", rename_all = "snake_case")]
pub enum RunTrigger {
    /// Its due time came.
    Due,
    /// Due times were missed; this one run stands for them.
    CaughtUp {
        /// How many, counted up to 1,000.
        missed: u32,
    },
    /// Someone asked (`schedule.run`).
    Asked {
        /// Who.
        by: Creator,
    },
    /// A trigger a newer node has.
    #[serde(other)]
    Unknown,
}

/// What a run did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RunOutcome {
    /// Being made: recorded, with the schedule's next due time, before
    /// anything is sent, so a node stopped meanwhile never makes it again.
    Making,
    /// A new session, its prompt sent, its first turn under way.
    Started {
        /// The session.
        session: SessionId,
    },
    /// A new session's first turn waits on a permission; nobody is asked
    /// questions in a scheduled session, so a person answers or archives
    /// it.
    Waiting {
        /// The session.
        session: SessionId,
    },
    /// A new session's first turn ended, or the session did.
    Finished {
        /// The session.
        session: SessionId,
        /// How.
        end: RunEnd,
    },
    /// The prompt went into an existing session's queue.
    Prompted {
        /// The session.
        session: SessionId,
        /// Whether it started a turn or waits.
        delivered: Delivered,
    },
    /// Nothing was sent.
    Skipped {
        /// Why.
        reason: SkipReason,
    },
    /// The call the run made was refused.
    Failed {
        /// The refusal.
        error: RunError,
        /// The session it had started, if it got that far.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session: Option<SessionId>,
    },
    /// The node stopped while it was being made; it is not made again.
    Interrupted,
    /// An outcome a newer node has.
    #[serde(other)]
    Unknown,
}

impl RunOutcome {
    /// The longest message a failed run keeps, in bytes.
    pub const MESSAGE_MAX: usize = 512;

    /// Whether the run counts toward `max_runs`: it sent its prompt.
    #[must_use]
    pub const fn counts(&self) -> bool {
        match self {
            Self::Started { .. }
            | Self::Waiting { .. }
            | Self::Finished { .. }
            | Self::Prompted { .. } => true,
            Self::Making
            | Self::Skipped { .. }
            | Self::Failed { .. }
            | Self::Interrupted
            | Self::Unknown => false,
        }
    }
}

/// A run's refused call: the code it answered and its message.
///
/// Not a [`WireError`]: its [`ErrorCode`] has no `unknown`, so one code a
/// newer node adds would make a whole list of runs unreadable to an older
/// client (the contract's review, item 16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunError {
    /// What kind of refusal.
    pub code: RunErrorCode,
    /// Why, cut at [`RunOutcome::MESSAGE_MAX`] bytes.
    pub message: String,
}

impl RunError {
    /// `error` as a run keeps it, its message cut on a character boundary.
    #[must_use]
    pub fn from_wire(error: &WireError) -> Self {
        let mut end = error.message.len().min(RunOutcome::MESSAGE_MAX);
        while !error.message.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        Self {
            code: error.code.into(),
            message: error.message.get(..end).unwrap_or_default().to_owned(),
        }
    }
}

/// [`ErrorCode`], and `unknown` for a code a newer node has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RunErrorCode {
    /// [`ErrorCode::InvalidRequest`].
    InvalidRequest,
    /// [`ErrorCode::Unauthorized`].
    Unauthorized,
    /// [`ErrorCode::Forbidden`].
    Forbidden,
    /// [`ErrorCode::ProtocolMismatch`].
    ProtocolMismatch,
    /// [`ErrorCode::NotFound`].
    NotFound,
    /// [`ErrorCode::Conflict`].
    Conflict,
    /// [`ErrorCode::Unsupported`].
    Unsupported,
    /// [`ErrorCode::HarnessNotInstalled`].
    HarnessNotInstalled,
    /// [`ErrorCode::SpawnFailed`].
    SpawnFailed,
    /// [`ErrorCode::Unavailable`].
    Unavailable,
    /// [`ErrorCode::Internal`].
    Internal,
    /// A code a newer node has.
    #[serde(other)]
    Unknown,
}

impl From<ErrorCode> for RunErrorCode {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::InvalidRequest => Self::InvalidRequest,
            ErrorCode::Unauthorized => Self::Unauthorized,
            ErrorCode::Forbidden => Self::Forbidden,
            ErrorCode::ProtocolMismatch => Self::ProtocolMismatch,
            ErrorCode::NotFound => Self::NotFound,
            ErrorCode::Conflict => Self::Conflict,
            ErrorCode::Unsupported => Self::Unsupported,
            ErrorCode::HarnessNotInstalled => Self::HarnessNotInstalled,
            ErrorCode::SpawnFailed => Self::SpawnFailed,
            ErrorCode::Unavailable => Self::Unavailable,
            ErrorCode::Internal => Self::Internal,
        }
    }
}

/// How a run's new session's first turn ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "end", rename_all = "snake_case")]
pub enum RunEnd {
    /// The turn ended.
    Turn {
        /// Why.
        reason: StopReason,
    },
    /// The turn failed.
    Failed {
        /// The agent's reason, cut at [`RunOutcome::MESSAGE_MAX`] bytes.
        message: String,
    },
    /// The session ended before its turn did.
    Session {
        /// Its final status.
        status: SessionStatus,
    },
    /// An end a newer node has.
    #[serde(other)]
    Unknown,
}

/// Why a run sent nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum SkipReason {
    /// The previous run is being made, or its session is still live
    /// (`keep`) or in its first turn (`archive`).
    PreviousRunning,
    /// The previous run's prompt still waits in the session's queue.
    PreviousWaiting,
    /// The session's last turn was stopped and nothing was sent to it
    /// since (its board says `interrupted`): nothing automatic starts a
    /// session a person stopped.
    Stopped,
    /// Due times passed while the node could not run them, and were not
    /// caught up (`catch_up: skip`, or the latest more than 24 hours ago).
    Missed {
        /// How many, counted up to 1,000.
        count: u32,
    },
    /// A reason a newer node has.
    #[serde(other)]
    Unknown,
}

/// `schedule.runs`: a schedule's newest runs. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListRuns {
    /// Which schedule.
    pub schedule: ScheduleId,
}

/// Reply to `schedule.runs`: at most [`RunList::MAX`], newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunList {
    /// The runs.
    pub runs: Vec<ScheduleRun>,
}

impl RunList {
    /// The runs kept per schedule.
    pub const MAX: usize = 50;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn outcomes_have_their_wire_names() {
        let session = SessionId::random();
        let failed = RunOutcome::Failed {
            error: RunError::from_wire(&WireError::new(ErrorCode::Forbidden, "no")),
            session: None,
        };
        assert_eq!(
            serde_json::to_string(&failed).unwrap(),
            r#"{"outcome":"failed","error":{"code":"forbidden","message":"no"}}"#
        );
        let finished = RunOutcome::Finished {
            session,
            end: RunEnd::Turn {
                reason: StopReason::EndTurn,
            },
        };
        let json = serde_json::to_string(&finished).unwrap();
        assert!(
            json.contains(r#""end":{"end":"turn","reason":"end_turn"}"#),
            "{json}"
        );
        assert_eq!(serde_json::from_str::<RunOutcome>(&json).unwrap(), finished);
        let skipped: RunOutcome =
            serde_json::from_str(r#"{"outcome":"skipped","reason":{"reason":"missed","count":4}}"#)
                .unwrap();
        assert_eq!(
            skipped,
            RunOutcome::Skipped {
                reason: SkipReason::Missed { count: 4 }
            }
        );
        let newer: RunOutcome = serde_json::from_str(r#"{"outcome":"delegated"}"#).unwrap();
        assert_eq!(newer, RunOutcome::Unknown);
        assert!(!newer.counts() && finished.counts());
    }

    #[test]
    fn a_run_error_keeps_a_bounded_message_and_reads_newer_codes() {
        let long = "é".repeat(400);
        let error = RunError::from_wire(&WireError::new(ErrorCode::Internal, long));
        assert!(error.message.len() <= RunOutcome::MESSAGE_MAX);
        assert!(error.message.chars().all(|character| character == 'é'));
        let newer: RunError =
            serde_json::from_str(r#"{"code":"quota_exceeded","message":"x"}"#).unwrap();
        assert_eq!(newer.code, RunErrorCode::Unknown);
    }

    #[test]
    fn run_numbers_count_up() {
        assert_eq!(RunNumber::FIRST.next().unwrap().get(), 2);
        assert_eq!(RunNumber(u32::MAX).next(), None);
        assert_eq!(RunNumber::new(0), None);
        assert!(serde_json::from_str::<RunNumber>("0").is_err());
    }
}
