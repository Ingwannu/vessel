//! `agent.status` (protocol 11, ADR 0032): whether an agent is installed
//! and signed in, under one profile, as typed facts with the reason when
//! a fact is not known. Found without a turn: the agent's own status
//! command (Claude Code, Codex) and its ACP handshake, each only when a
//! person asks (`refresh`).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::added::SignInHelp;
use super::name::HarnessRef;
use super::text::{AgentProgram, AgentVersion};
use crate::harness::StructuredSupport;
use crate::model::DurationMs;
use crate::name::ProfileName;

/// `agent.status`: the facts of every agent and profile, or of one agent
/// (and one of its profiles). Needs `read`; `refresh`, which runs checks,
/// is refused to an agent and needs `agent`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetAgentStatus {
    /// Only this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<HarnessRef>,
    /// Only this profile of it (its default without one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// Check now: run its status command and its handshake (no prompt).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub refresh: bool,
}

/// One agent under one profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentReadiness {
    /// Which agent.
    pub agent: HarnessRef,
    /// Which profile (the agent's default has the agent's name).
    pub profile: ProfileName,
    /// Whether it is installed.
    pub installed: Installed,
    /// Whether it is signed in.
    pub signed_in: SignedIn,
    /// When the older of the two facts was found.
    pub checked_at: DateTime<Utc>,
}

/// Whether an agent is installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Installed {
    /// Found, and (when its handshake was seen) speaking Vessel's ACP.
    Found {
        /// Where.
        path: PathBuf,
        /// Its version: the four's `--version`, an added agent's own word
        /// in its handshake (never run with `--version`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<AgentVersion>,
        /// Whether it runs conversations here.
        structured: StructuredSupport,
    },
    /// Not found.
    Missing {
        /// The program looked for.
        program: AgentProgram,
    },
    /// Its handshake speaks an older ACP version than Vessel.
    TooOld {
        /// Where.
        path: PathBuf,
        /// Its version, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<AgentVersion>,
        /// The ACP version it speaks.
        speaks: u16,
        /// The one Vessel speaks.
        needs: u16,
    },
    /// Its handshake speaks a newer ACP version than Vessel.
    TooNew {
        /// Where.
        path: PathBuf,
        /// Its version, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<AgentVersion>,
        /// The ACP version it speaks.
        speaks: u16,
        /// The one Vessel speaks.
        needs: u16,
    },
    /// Not known.
    Unknown {
        /// Why.
        reason: UnknownReason,
    },
}

/// Whether an agent is signed in under a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SignedIn {
    /// Its own status command says it holds credentials (Claude Code's
    /// `loggedIn: true`, Codex's "Logged in"). Whether they are still
    /// valid is known at the first turn.
    Yes,
    /// Its status command, or `auth_required` in its handshake, says not.
    No {
        /// How a person signs in.
        help: SignInHelp,
    },
    /// No sign-in is needed: the profile runs on a model route (ADR 0015).
    NotRequired,
    /// Not known.
    Unknown {
        /// Why.
        reason: UnknownReason,
    },
}

/// Why a fact is not known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum UnknownReason {
    /// The agent is not installed.
    NotInstalled,
    /// Not asked yet: a status command and a handshake run only on
    /// `refresh`.
    NotChecked,
    /// A session of this profile is running; a check could race it for
    /// the credentials (refreshing a token), so none was run.
    InUse,
    /// The handshake completed without asking for a sign-in: many agents
    /// ask only at the first prompt.
    NotSaid,
    /// The profile's home variable is relative: whose sign-in is unknown.
    NoHome,
    /// The check answered in words Vessel does not read (another
    /// version's).
    Unread,
    /// The check did not answer in time.
    TimedOut {
        /// The limit.
        after: DurationMs,
    },
    /// The check or the handshake failed: at which step, never its
    /// output.
    Failed {
        /// Where.
        step: CheckStep,
    },
}

/// Where a check failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CheckStep {
    /// The program did not start.
    Spawn,
    /// `initialize` failed.
    Initialize,
    /// `session/new` failed (not for a sign-in).
    SessionNew,
    /// It exited before answering.
    Exited,
}
