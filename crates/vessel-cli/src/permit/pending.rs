//! Pending rows reuse the durable inbox's exact source and request identities.
//! Compact forms remain visible; fetching their full form is a mutation preflight.

use serde::Serialize;
use std::fmt;
use vessel_proto::inbox::{InboxEntry, InboxItem, InboxKey, InboxRequest};
use vessel_proto::{PermissionId, QuestionId, SessionId};

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
    #[serde(skip)]
    pub(crate) source: InboxKey,
}

/// Unknown inbox kinds cannot silently disappear from a supposedly complete list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("this node returned an unsupported pending-entry kind; update the client")]
pub(crate) struct UnsupportedKind;

impl Row {
    /// Project authoritative pending entries, including payload-budget references.
    /// Known outcome-only items are not pending permissions.
    pub(crate) fn of(entry: &InboxEntry) -> Result<Option<Self>, UnsupportedKind> {
        let (id, name, description) = match &entry.item {
            InboxItem::Permission { request, tool, .. } => (
                PendingId::Permission(request.id),
                tool.as_ref()
                    .map_or_else(|| "Permission".to_owned(), |tool| tool.title.clone()),
                String::new(),
            ),
            InboxItem::Question { question } => (
                PendingId::Question(question.id),
                "Question".to_owned(),
                question.message.clone(),
            ),
            InboxItem::InSession { request } => match request {
                InboxRequest::Permission { id } => (
                    PendingId::Permission(*id),
                    "Permission".to_owned(),
                    "Details in session".to_owned(),
                ),
                InboxRequest::Question { id } => (
                    PendingId::Question(*id),
                    "Question".to_owned(),
                    "Details in session".to_owned(),
                ),
            },
            InboxItem::Finished | InboxItem::Failed | InboxItem::Limit => return Ok(None),
            InboxItem::Unknown => return Err(UnsupportedKind),
        };
        Ok(Some(Self {
            id,
            agent_id: entry.key.session,
            agent_short_id: entry
                .key
                .session
                .to_string()
                .chars()
                .take(AGENT_PREFIX)
                .collect(),
            name,
            description,
            source: entry.key,
        }))
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
