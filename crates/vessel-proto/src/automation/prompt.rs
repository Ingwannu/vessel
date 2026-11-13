//! Saved prompts (protocol 11, ADR 0033): a person's text, named, for a
//! project or for the machine, with what a session started from one runs
//! with.
//!
//! [Decision Log]
//! - Purpose: agent-orchestrator's Cues (the one reference with saved
//!   prompts): text a person reuses, sent to a session or used to start
//!   one.
//! - Kept by the node, never read from a file in the repository: a prompt
//!   steers an agent that may be running without asking, so text a
//!   repository supplies would be someone else's instructions in the
//!   sense ADR 0027 gives commands; the node would need a second approval
//!   flow for it. agent-orchestrator keeps Cues in its database too.
//! - A person's text: agents may list and read them, not write them. A
//!   saved prompt is later sent by a person who may not read it again; an
//!   agent able to change one could put its words in that person's mouth.
//! - What a session started from one runs with is a `SessionSpec`, what
//!   `session.create` takes, so nothing here restates its rules.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::text::{PromptName, PromptText};
use crate::id::{ProjectId, SavedPromptId};
use crate::profile::Creator;
use crate::session::SessionSpec;

/// A saved prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SavedPrompt {
    /// Its id.
    pub id: SavedPromptId,
    /// Its name, unique within its project (or the machine's own).
    pub name: PromptName,
    /// The text.
    pub text: PromptText,
    /// The project it belongs to; absent: the machine's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
    /// What a session started from it runs with (structured, without
    /// `resume`; its place, if any, in the prompt's project).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Box<SessionSpec>>,
    /// Who last wrote it.
    pub defined_by: Creator,
    /// When.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
    /// Changes at every change; `prompt.update` names the one it was made
    /// from.
    pub revision: u32,
}

impl SavedPrompt {
    /// The most saved prompts a node keeps.
    pub const MAX: usize = 256;
}

/// `prompt.list`: the saved prompts, by name. Needs `read`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListPrompts {
    /// Only this project's and the machine's; every one when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
}

/// Reply to `prompt.list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptList {
    /// The prompts, by name.
    pub prompts: Vec<SavedPrompt>,
}

/// `prompt.create`: save a prompt. Needs `operate`; agents are refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreatePrompt {
    /// Its name.
    pub name: PromptName,
    /// The text.
    pub text: PromptText,
    /// Its project; absent: the machine's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
    /// What a session started from it runs with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Box<SessionSpec>>,
}

/// `prompt.update`: change a saved prompt (its project stays). Needs
/// `operate`; agents are refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdatePrompt {
    /// Which.
    pub prompt: SavedPromptId,
    /// The revision the change was made from; another is `conflict`.
    pub revision: u32,
    /// Its name.
    pub name: PromptName,
    /// The text.
    pub text: PromptText,
    /// What a session started from it runs with; absent clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Box<SessionSpec>>,
}

/// `prompt.delete`: remove a saved prompt. Needs `operate`; agents are
/// refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeletePrompt {
    /// Which.
    pub prompt: SavedPromptId,
}
