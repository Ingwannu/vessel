//! Canonical read-only pending lookup, including archived/stored sessions.
//! Inbox is an attention projection and deliberately suppresses archived rows.
//! Existing session.list/conversation calls do not start a provider or a turn.
use super::pending::{PendingId, Row};
use std::collections::{BTreeMap, HashSet};
use vessel_client::{CallError, NodeClient};
use vessel_proto::conversation::Conversation;
use vessel_proto::{PermissionRequest, Question, SessionId, SessionInfo, SessionMode};

mod budget;
const MAX_SESSIONS: usize = 10_000;
const MAX_PENDING_ROWS: usize = 10_000;

/// One canonical node read boundary shared by the client and in-memory fixture.
pub(crate) trait Source {
    async fn sessions(&mut self) -> Result<Vec<SessionInfo>, CallError>;
    async fn conversation(&mut self, session: SessionId) -> Result<Conversation, CallError>;
}
impl Source for NodeClient {
    async fn sessions(&mut self) -> Result<Vec<SessionInfo>, CallError> {
        NodeClient::sessions(self).await
    }
    async fn conversation(&mut self, session: SessionId) -> Result<Conversation, CallError> {
        NodeClient::conversation(self, session).await
    }
}

#[derive(thiserror::Error)]
pub(crate) enum SourceError {
    #[error("selected forms exceed this command's payload budget; select fewer requests")]
    Budget,
    #[error("pending requests exceed this command's collection bound")]
    Limit,
    #[error("the selected request forms could not be encoded")]
    Encoding(#[source] serde_json::Error),
    // Retain typed peer detail without exposing its arbitrary diagnostic chain.
    #[error("could not read pending requests from the selected node")]
    Node(CallError),
    #[error("a selected request was resolved or withdrawn; list pending requests again")]
    Gone,
    #[error("the node returned a duplicate session or pending request")]
    Duplicate,
    #[error("selected requests do not belong to the same agent; no answer was sent")]
    Mismatch,
}
impl From<CallError> for SourceError {
    fn from(error: CallError) -> Self {
        Self::Node(error)
    }
}
impl std::fmt::Debug for SourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Budget => formatter.write_str("Budget"),
            Self::Limit => formatter.write_str("Limit"),
            Self::Encoding(error) => formatter.debug_tuple("Encoding").field(error).finish(),
            Self::Gone => formatter.write_str("Gone"),
            Self::Duplicate => formatter.write_str("Duplicate"),
            Self::Mismatch => formatter.write_str("Mismatch"),
            Self::Node(error) => {
                let code = match error {
                    CallError::Remote(error) => Some(error.code),
                    CallError::Disconnected
                    | CallError::UnexpectedReply(_)
                    | CallError::RequestIdsExhausted
                    | CallError::NotInProtocol { .. }
                    | CallError::UnassignedProtocol { .. } => None,
                };
                formatter.debug_struct("Node").field("code", &code).finish()
            }
        }
    }
}

/// Complete current bodies, with their actual session rather than a fake InboxKey.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Ready {
    Permission {
        session: SessionId,
        request: PermissionRequest,
    },
    Question {
        session: SessionId,
        question: Question,
    },
}

impl std::fmt::Debug for Ready {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (session, request) = match self {
            Self::Permission { session, request } => (session, PendingId::Permission(request.id)),
            Self::Question { session, question } => (session, PendingId::Question(question.id)),
        };
        formatter
            .debug_struct("Ready")
            .field("session", session)
            .field("request", &request)
            .finish_non_exhaustive()
    }
}

/// Every structured session participates, including archived or ended ones.
/// No list is published from an incomplete or failed traversal.
pub(crate) async fn list(source: &mut impl Source) -> Result<Vec<Row>, SourceError> {
    let sessions = source.sessions().await?;
    if sessions.len() > MAX_SESSIONS {
        return Err(SourceError::Limit);
    }
    let mut seen_sessions = HashSet::new();
    let mut seen_requests = HashSet::new();
    let mut rows = Vec::new();
    let mut budget = budget::Budget::default();
    for session in sessions {
        if !seen_sessions.insert(session.id) {
            return Err(SourceError::Duplicate);
        }
        match session.spec.mode {
            SessionMode::Terminal { .. } => continue,
            SessionMode::Structured { .. } => {}
        }
        let held = source.conversation(session.id).await?;
        for request in held.permissions {
            budget.charge(&request)?;
            append(
                &mut rows,
                &mut seen_requests,
                Row::permission(session.id, &request),
            )?;
        }
        for question in held.questions {
            budget.charge(&question)?;
            append(
                &mut rows,
                &mut seen_requests,
                Row::question(session.id, &question),
            )?;
        }
    }
    Ok(rows)
}
fn append(
    rows: &mut Vec<Row>,
    seen: &mut HashSet<(SessionId, PendingId)>,
    row: Row,
) -> Result<(), SourceError> {
    if rows.len() >= MAX_PENDING_ROWS {
        return Err(SourceError::Limit);
    }
    if !seen.insert((row.agent_id, row.id)) {
        return Err(SourceError::Duplicate);
    }
    rows.push(row);
    Ok(())
}

/// Selection is agent-scoped. Re-read that exact canonical snapshot once and
/// recover every selected full body before the batch can perform any effects.
pub(crate) async fn preflight(
    source: &mut impl Source,
    rows: &[Row],
) -> Result<Vec<Ready>, SourceError> {
    let Some(first) = rows.first() else {
        return Ok(Vec::new());
    };
    if rows.len() > MAX_PENDING_ROWS {
        return Err(SourceError::Limit);
    }
    if rows.iter().any(|row| row.agent_id != first.agent_id) {
        return Err(SourceError::Mismatch);
    }
    let mut selected_ids = HashSet::new();
    if rows.iter().any(|row| !selected_ids.insert(row.id)) {
        return Err(SourceError::Duplicate);
    }
    let held = source.conversation(first.agent_id).await?;
    let count = held
        .permissions
        .len()
        .checked_add(held.questions.len())
        .ok_or(SourceError::Limit)?;
    if count > MAX_PENDING_ROWS {
        return Err(SourceError::Limit);
    }
    let mut permissions = unique(held.permissions, |request| request.id)?;
    let mut questions = unique(held.questions, |question| question.id)?;
    let mut ready = Vec::with_capacity(rows.len());
    let mut budget = budget::Budget::default();
    for row in rows {
        let item = match row.id {
            PendingId::Permission(id) => {
                let request = permissions.remove(&id).ok_or(SourceError::Gone)?;
                budget.charge(&request)?;
                Ready::Permission {
                    session: row.agent_id,
                    request,
                }
            }
            PendingId::Question(id) => {
                let question = questions.remove(&id).ok_or(SourceError::Gone)?;
                budget.charge(&question)?;
                Ready::Question {
                    session: row.agent_id,
                    question,
                }
            }
        };
        ready.push(item);
    }
    Ok(ready)
}
// An ephemeral index of one already-bounded reply; it owns/moves complete forms
// rather than cloning payloads or accepting a duplicate identity's first body.
fn unique<Id: Ord, Form>(
    forms: Vec<Form>,
    id: impl Fn(&Form) -> Id,
) -> Result<BTreeMap<Id, Form>, SourceError> {
    let mut indexed = BTreeMap::new();
    for form in forms {
        if indexed.insert(id(&form), form).is_some() {
            return Err(SourceError::Duplicate);
        }
    }
    Ok(indexed)
}

#[cfg(test)]
mod tests;
