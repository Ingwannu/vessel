//! `agent.diagnostic` (ADR 0111, TRACKER 568 and 67): one agent's
//! diagnostic, as Paseo's provider diagnostic (`provider_diagnostic_request`,
//! `ProviderSnapshotManager.getProviderDiagnostic`), with a new probe of its
//! catalogue (TRACKER 87's "diagnostic refresh").
//!
//! [Decision Log]
//! - Purpose: a person asks "why does this agent not work here" and gets one
//!   block to read or paste: which program runs, where it is, its version,
//!   whether it runs conversations and is signed in, what a probe of it said
//!   and how many models it offers.
//! - Constraint: typed facts, rendered by each client into Paseo's text
//!   ([`AgentDiagnostic::text`]), as `node.diagnostics` is (ADR 0074): no
//!   field carries the node's environment, a shell's output or a profile's
//!   secrets. Paths are the ones `agent.status` already gives, with the
//!   home folded to `~` by the node.
//! - Alternatives: Paseo's text built by the node (pattern redaction after
//!   the fact misses what it does not foresee); Paseo's PATH, shell and
//!   `which -a` rows (environment values and shell output). The search
//!   path's matches are kept, as paths only.
//! - Bounds: the node keeps a title to [`MAX_TITLE`], each text field to
//!   [`MAX_TEXT`] and the matches to [`MAX_MATCHES`]; clients render
//!   whatever arrives.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::name::HarnessRef;
use super::status::{Installed, SignedIn};
use super::text::AgentVersion;
use crate::model::DurationMs;
use crate::name::ProfileName;

mod text;

/// The most bytes of an agent's title the node sends.
pub const MAX_TITLE: usize = 128;
/// The most bytes of a reason or message the node sends.
pub const MAX_TEXT: usize = 4096;
/// The most search-path matches the node sends.
pub const MAX_MATCHES: usize = 8;

/// `agent.diagnostic`: `agent`'s diagnostic under `profile` (its default
/// when `None`). Needs `read`; refused to an agent, since it runs the
/// agent's checks, as a refresh of `agent.status` is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct GetAgentDiagnostic {
    /// Which agent.
    pub agent: HarnessRef,
    /// Which of its profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
}

/// One agent's diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentDiagnostic {
    /// Which agent.
    pub agent: HarnessRef,
    /// Under which profile.
    pub profile: ProfileName,
    /// What it is called (Paseo's label; an added agent's `<label> (ACP)`).
    pub title: String,
    /// The program that runs.
    pub command: DiagnosticCommand,
    /// Every match for the program on the node's search path, in search
    /// order, the home folded to `~` (at most [`MAX_MATCHES`]).
    pub matches: Vec<PathBuf>,
    /// Where it is, its version and whether it runs conversations here.
    pub installed: Installed,
    /// Whether it is signed in.
    pub signed_in: SignedIn,
    /// What a new probe of it said.
    pub probe: AgentProbe,
    /// Its catalogue after that probe.
    pub status: CatalogStatus,
    /// When the diagnostic was made.
    pub checked_at: DateTime<Utc>,
}

/// The program an agent runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiagnosticCommand {
    /// Where the command comes from.
    pub source: CommandSource,
    /// The program, as configured (a name looked up on the search path, or
    /// a path).
    pub program: String,
    /// Its arguments: an added agent's, as `agent.list` gives them; none
    /// for a built-in agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

/// Where an agent's command comes from (Paseo's "Command source").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CommandSource {
    /// Vessel's own default for one of its built-in agents.
    BuiltIn,
    /// An agent added to this node by its command (ADR 0032).
    Added,
}

/// What a probe of an agent said, step by step (Paseo's ACP probe rows).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentProbe {
    /// Starting its process.
    pub spawn: ProbeStep,
    /// The ACP handshake (`initialize`).
    pub initialize: ProbeStep,
    /// The session the probe opens to read the agent's options
    /// (`session/new`).
    pub session: ProbeStep,
    /// The version the agent gave in its handshake, when it gave one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<AgentVersion>,
    /// How long the probe took.
    pub took: DurationMs,
}

/// One step of a probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProbeStep {
    /// It completed.
    Ok,
    /// It failed, saying why (at most [`MAX_TEXT`] bytes).
    Failed {
        /// Why.
        reason: String,
    },
    /// An earlier step failed, or there was nothing to start.
    NotReached,
}

/// An agent's catalogue (Paseo's "Status" and "Models" lines).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CatalogStatus {
    /// It offered its options.
    Ready {
        /// How many models it offers (its model option's values).
        models: u32,
    },
    /// The probe failed (its message, at most [`MAX_TEXT`] bytes).
    Error {
        /// What failed.
        message: String,
    },
    /// Nothing to ask: it is not installed, or runs no conversations here.
    Unavailable,
}

/// `text` cut to at most `max` bytes, on a character boundary.
#[must_use]
pub fn bounded_text(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests;
