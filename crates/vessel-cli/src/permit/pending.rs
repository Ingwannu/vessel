//! Pending rows carry canonical session/request identity, independent of attention.
//! Archiving suppresses Inbox rows but must not hide retained session questions.

use serde::Serialize;
use std::fmt;
use vessel_proto::{PermissionId, PermissionRequest, Question, QuestionId, SessionId};

/// Paseo's displayed agent prefix; full typed identities stay in structured output.
const AGENT_PREFIX: usize = 7;
/// Paseo's displayed request prefix, never used as the request sent to a node.
const REQUEST_PREFIX: usize = 8;

/// Questions and permissions share a CLI list without erasing their id types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub(crate) enum PendingId {
    Permission(PermissionId),
    Question(QuestionId),
}

impl fmt::Display for PendingId {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permission(id) => fmt::Display::fmt(id, output),
            Self::Question(id) => fmt::Display::fmt(id, output),
        }
    }
}

/// Public command output retains ids and never embeds a grant's opaque input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Row {
    pub(crate) id: PendingId,
    pub(crate) agent_id: SessionId,
    pub(crate) agent_short_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
}

impl Row {
    pub(crate) fn permission(session: SessionId, request: &PermissionRequest) -> Self {
        Self::pending(
            session,
            PendingId::Permission(request.id),
            "Permission",
            "-",
        )
    }
    pub(crate) fn question(session: SessionId, question: &Question) -> Self {
        Self::pending(
            session,
            PendingId::Question(question.id),
            "Question",
            &question.message,
        )
    }
    fn pending(session: SessionId, id: PendingId, name: &str, description: &str) -> Self {
        Self {
            id,
            agent_id: session,
            agent_short_id: session.to_string().chars().take(AGENT_PREFIX).collect(),
            name: name.to_owned(),
            description: description.to_owned(),
        }
    }

    /// A table caption only; lookup and mutation retain the full typed request id.
    pub(crate) fn short_request(&self) -> String {
        self.id.to_string().chars().take(REQUEST_PREFIX).collect()
    }
}

// Presentation belongs to W17's shared renderer. This mapping owns only the
// permit columns and the exact identity selected by quiet/structured output.
impl crate::parity::output::Row for Row {
    fn id(&self) -> String {
        self.id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["AGENT", "REQ_ID", "TOOL", "DESCRIPTION"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.agent_short_id.clone(),
            self.short_request(),
            self.name.clone(),
            self.description.clone(),
        ]
    }
}

#[cfg(test)]
mod tests;
