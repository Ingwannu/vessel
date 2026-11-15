//! Read-only permit preflight, using the node-owned inbox rather than a parallel
//! session scan. Complete listing precedes selection; exact live forms precede
//! any eventual mutation. No generic session-input fallback lives here.
use super::{
    pages::{Pages, ReadError},
    pending::{PendingId, Row},
};
use vessel_client::{CallError, NodeClient};
use vessel_proto::inbox::{InboxEntry, InboxItem, InboxKey, InboxPage, ListInbox};
use vessel_proto::{PermissionRequest, Question};

mod budget;

/// The existing node calls, shared by the real client and in-memory tests.
pub(crate) trait Source {
    async fn page(&mut self, request: ListInbox) -> Result<InboxPage, CallError>;
    async fn live(&mut self, key: InboxKey) -> Result<Option<InboxEntry>, CallError>;
}
impl Source for NodeClient {
    async fn page(&mut self, request: ListInbox) -> Result<InboxPage, CallError> {
        self.inbox(request).await
    }
    async fn live(&mut self, key: InboxKey) -> Result<Option<InboxEntry>, CallError> {
        self.inbox_request(key).await
    }
}

/// Preflight errors occur before the response path has any effects.
#[derive(thiserror::Error)]
pub(crate) enum SourceError {
    #[error("selected forms exceed this command's payload budget; select fewer requests")]
    Budget,
    #[error("the selected request forms could not be encoded")]
    Encoding(#[source] serde_json::Error),
    #[error(transparent)]
    Pages(#[from] ReadError),
    // The original typed error remains available for closed-code projection.
    // A peer's arbitrary message must not enter the automatic CLI cause chain.
    #[error("could not read pending requests from the selected node")]
    Node(CallError),
    #[error("a selected request was resolved or withdrawn; list pending requests again")]
    Gone,
    #[error("the complete request exceeds the node's reply budget; open its session")]
    Compact,
    #[error("the node returned a different request for the selected source; no answer was sent")]
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
            Self::Encoding(error) => formatter.debug_tuple("Encoding").field(error).finish(),
            Self::Pages(error) => formatter.debug_tuple("Pages").field(error).finish(),
            Self::Gone => formatter.write_str("Gone"),
            Self::Compact => formatter.write_str("Compact"),
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

/// A complete, current form with its authoritative source key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ready {
    Permission {
        source: InboxKey,
        request: PermissionRequest,
    },
    Question {
        source: InboxKey,
        question: Question,
    },
}

/// No output is published from an incomplete or failed traversal.
pub(crate) async fn list(source: &mut impl Source) -> Result<Vec<Row>, SourceError> {
    let mut pages = Pages::default();
    while let Some(request) = pages.request()? {
        pages.accept(source.page(request).await?)?;
    }
    Ok(pages.finish()?)
}

/// Recover and verify every selected full form before a batch may begin.
/// A changed/compact item never silently receives a blind grant.
pub(crate) async fn preflight(
    source: &mut impl Source,
    rows: &[Row],
) -> Result<Vec<Ready>, SourceError> {
    let mut ready = Vec::with_capacity(rows.len());
    let mut budget = budget::Budget::default();
    for row in rows {
        let entry = source.live(row.source).await?.ok_or(SourceError::Gone)?;
        if entry.key != row.source {
            return Err(SourceError::Mismatch);
        }
        let item = match entry.item {
            InboxItem::Permission { request, .. } => {
                if row.id != PendingId::Permission(request.id) {
                    return Err(SourceError::Mismatch);
                }
                budget.charge(&request)?;
                Ready::Permission {
                    source: entry.key,
                    request,
                }
            }
            InboxItem::Question { question } => {
                if row.id != PendingId::Question(question.id) {
                    return Err(SourceError::Mismatch);
                }
                budget.charge(&question)?;
                Ready::Question {
                    source: entry.key,
                    question,
                }
            }
            InboxItem::InSession { .. } => return Err(SourceError::Compact),
            InboxItem::Finished | InboxItem::Failed | InboxItem::Limit | InboxItem::Unknown => {
                return Err(SourceError::Gone);
            }
        };
        ready.push(item);
    }
    Ok(ready)
}

#[cfg(test)]
mod tests;
