//! Terminals beside sessions (protocol 11, ADR 0031): a person's or an
//! agent's shell in a session's folder or a project's, and the runs of a
//! project's scripts and services.
//!
//! [Decision Log]
//! - Purpose: the three references open shells beside an agent's session
//!   that outlive the page, and run a project's scripts in the same kind of
//!   view; until protocol 11 Vessel's only terminal was the agent's own (a
//!   session in terminal mode).
//! - Alternatives: a shell as a kind of session (it would be a card on the
//!   board, in every older client's session list with a spec it cannot
//!   decode, in an agent's session count, exported and archived, and its
//!   output kept in the store); a terminal transport of its own.
//! - Choice: a thing of its own (`TerminalId`, listed and limited apart),
//!   carried by the sessions' terminal transport: its log is the
//!   `EventRecord`s a terminal session's log is made of, followed through
//!   the same frames and paged with the same `EventPage`, and its input is
//!   the same `TerminalInput`. ADR 0031 says why.
//! - Cost: thirteen calls of their own, and a terminal is kept in the
//!   node's memory only (it ends with the node).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::{EventSeq, ProjectId, SessionId, TerminalId};
use crate::profile::Creator;
use crate::scripts::ScriptName;
use crate::session::SessionStatus;
use crate::terminal::TerminalSize;
use crate::title::SessionTitle;
use crate::workspace::{ProjectCommand, RelPath};

mod address;
mod capture;
mod requests;
pub use address::TerminalAddress;
pub use capture::{
    CaptureLine, CaptureLineCount, CaptureOptions, CaptureRange, CaptureStyle, CaptureTerminal,
    TerminalCapture,
};

pub use requests::{
    CloseTerminal, CreateTerminal, GetTerminal, KillTerminal, ListTerminals, ReadTerminal,
    RenameTerminal, SendTerminal, SubscribeTerminal, TerminalSubscribed,
};

/// Where a terminal runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalPlace {
    /// Beside a session, in its folder (its `cwd`, or the node's home when
    /// it has none). Ended with the session's deletion.
    Session {
        /// Which.
        session: SessionId,
    },
    /// In a project's folder, or a folder in it. Ended with the project's
    /// removal.
    Project {
        /// Which.
        project: ProjectId,
        /// A folder inside the project's root; the root when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dir: Option<RelPath>,
    },
}

impl TerminalPlace {
    /// The session it is beside, if it is.
    #[must_use]
    pub const fn session(&self) -> Option<SessionId> {
        match self {
            Self::Session { session } => Some(*session),
            Self::Project { .. } => None,
        }
    }

    /// The project it is in, if it was placed in one.
    #[must_use]
    pub const fn project(&self) -> Option<ProjectId> {
        match self {
            Self::Project { project, .. } => Some(*project),
            Self::Session { .. } => None,
        }
    }
}

/// What a terminal runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalKind {
    /// The user's shell, in a pseudo-terminal; it takes input.
    Shell {
        /// The program started.
        program: PathBuf,
    },
    /// A run of a project's script: one command, to its end. No input.
    Script {
        /// Its name in `vessel.toml`.
        script: ScriptName,
        /// What runs.
        command: ProjectCommand,
    },
    /// A run of a project's service: one command, kept running. No input.
    Service {
        /// Its name in `vessel.toml`.
        script: ScriptName,
        /// What runs.
        command: ProjectCommand,
    },
    /// A kind a newer node knows.
    #[serde(other)]
    Unknown,
}

impl TerminalKind {
    /// Whether it takes terminal input (only a shell does).
    #[must_use]
    pub const fn takes_input(&self) -> bool {
        match self {
            Self::Shell { .. } => true,
            Self::Script { .. } | Self::Service { .. } | Self::Unknown => false,
        }
    }

    /// The script it runs, for a script's or a service's run.
    #[must_use]
    pub const fn script(&self) -> Option<&ScriptName> {
        match self {
            Self::Script { script, .. } | Self::Service { script, .. } => Some(script),
            Self::Shell { .. } | Self::Unknown => None,
        }
    }
}

/// A terminal as the node reports it.
///
/// `#[non_exhaustive]` with a constructor, for the reason `SessionInfo`
/// gives: a field added later would break every struct literal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[non_exhaustive]
pub struct TerminalInfo {
    /// Its id.
    pub id: TerminalId,
    /// Where it was placed.
    pub place: TerminalPlace,
    /// The absolute folder it started in.
    pub folder: PathBuf,
    /// What it runs.
    pub kind: TerminalKind,
    /// What a person called it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<SessionTitle>,
    /// Who opened it (ADR 0013).
    #[serde(default)]
    pub created_by: Creator,
    /// When it was opened.
    pub created_at: DateTime<Utc>,
    /// When its process ended for good, if it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// Its process's state: `running`, then `exited`, `failed` or
    /// `interrupted` (a service is `running` while it is kept running,
    /// restarts included).
    pub status: SessionStatus,
    /// A service's state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<ServiceState>,
    /// A shell's window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TerminalSize>,
    /// The oldest record still kept: older output was dropped (the
    /// scrollback is bounded).
    pub oldest_seq: EventSeq,
    /// The number the next record will get.
    pub next_seq: EventSeq,
    /// Changes with every field but the two sequence numbers, for
    /// `terminal.get`'s `changed_from`.
    pub revision: u64,
}

impl TerminalInfo {
    /// A terminal starting now: running, nothing recorded yet.
    #[must_use]
    pub fn new(
        id: TerminalId,
        place: TerminalPlace,
        folder: PathBuf,
        kind: TerminalKind,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            place,
            folder,
            kind,
            title: None,
            created_by: Creator::Owner,
            created_at,
            ended_at: None,
            status: SessionStatus::Running,
            service: None,
            size: None,
            oldest_seq: EventSeq::ZERO,
            next_seq: EventSeq::ZERO,
            revision: 0,
        }
    }

    /// Opened by `creator`.
    #[must_use]
    pub fn with_creator(mut self, creator: Creator) -> Self {
        self.created_by = creator;
        self
    }

    /// Called this.
    #[must_use]
    pub fn with_title(mut self, title: Option<SessionTitle>) -> Self {
        self.title = title;
        self
    }

    /// Whether its process may still run (or, for a service, be started
    /// again).
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.status.is_live()
    }
}

/// What a service is doing (protocol 11). Also a record of its log
/// (`SessionEvent::Service`), at every change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ServiceState {
    /// Starting its process.
    Starting {
        /// Which start, from 1.
        attempt: u32,
    },
    /// Its process runs.
    Running {
        /// Which start, from 1.
        attempt: u32,
        /// Since when.
        since: DateTime<Utc>,
    },
    /// Its process failed; it starts again at `restart_at`.
    BackingOff {
        /// The start that failed.
        attempt: u32,
        /// Its exit code (none for a signal, or a start that failed).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<i32>,
        /// When the next start is.
        restart_at: DateTime<Utc>,
    },
    /// It ended by itself with 0, and is not started again.
    Exited {
        /// The exit code (0).
        code: i32,
    },
    /// It is not started again, and why.
    Failed {
        /// Why.
        reason: ServiceFailure,
    },
    /// A person, its session's deletion, its project's removal or the
    /// node's stop ended it.
    Stopped,
    /// A state a newer node knows.
    #[serde(other)]
    Unknown,
}

impl ServiceState {
    /// A short line for a person, and for older clients' stand-in.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Starting { attempt } => format!("service starting (attempt {attempt})"),
            Self::Running { attempt, .. } => format!("service running (attempt {attempt})"),
            Self::BackingOff { attempt, code, .. } => match code {
                Some(code) => format!("service exited with {code} (attempt {attempt}); restarting"),
                None => format!("service failed (attempt {attempt}); restarting"),
            },
            Self::Exited { code } => format!("service exited with {code}"),
            Self::Failed { reason } => match reason {
                ServiceFailure::GaveUp { restarts } => {
                    format!("service failed {restarts} times in a row; given up")
                }
                ServiceFailure::NotApproved => {
                    "service not restarted: its commands are no longer approved".to_owned()
                }
                ServiceFailure::Spawn => "service could not be started".to_owned(),
                ServiceFailure::Unknown => "service failed".to_owned(),
            },
            Self::Stopped => "service stopped".to_owned(),
            Self::Unknown => "service changed".to_owned(),
        }
    }
}

/// Why a service is not started again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceFailure {
    /// It failed this many times in a row without running for a minute.
    GaveUp {
        /// Restarts tried.
        restarts: u32,
    },
    /// The set its command came from is no longer approved.
    NotApproved,
    /// Its process could not be started.
    Spawn,
    /// A reason a newer node knows.
    #[serde(other)]
    Unknown,
}

/// `terminal.list`'s answer: live terminals first, then kept ones, each
/// newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TerminalList {
    /// At most 96 (64 live, 32 ended).
    pub terminals: Vec<TerminalInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_a_newer_node_knows_reads_as_unknown() {
        let kind: TerminalKind = serde_json::from_str(r#"{"kind":"browser"}"#).unwrap();
        assert_eq!(kind, TerminalKind::Unknown);
        let state: ServiceState = serde_json::from_str(r#"{"state":"paused"}"#).unwrap();
        assert_eq!(state, ServiceState::Unknown);
    }

    #[test]
    fn a_place_reads_as_written() {
        let place: TerminalPlace = serde_json::from_str(
            r#"{"kind":"project","project":"7b0c2f4e-62a8-4f4c-9a51-8f6f4a3e2d10","dir":"web"}"#,
        )
        .unwrap();
        assert_eq!(place.session(), None);
        assert!(place.project().is_some());
        assert_eq!(
            serde_json::to_value(&place).unwrap()["dir"],
            serde_json::json!("web")
        );
    }

    #[test]
    fn only_a_shell_takes_input() {
        assert!(
            TerminalKind::Shell {
                program: PathBuf::from("/bin/sh")
            }
            .takes_input()
        );
        let command: ProjectCommand = "npm run dev".parse().unwrap();
        let script: ScriptName = "dev".parse().unwrap();
        assert!(!TerminalKind::Service { script, command }.takes_input());
        assert!(!TerminalKind::Unknown.takes_input());
    }
}
