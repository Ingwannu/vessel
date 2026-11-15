//! Complete the authoritative inbox traversal before publishing a permit list.
//! A malformed cursor or unknown kind cannot become a successful partial list.

use super::pending::{PendingId, Row, UnsupportedKind};
use std::collections::HashSet;
use vessel_proto::SessionId;
use vessel_proto::inbox::{InboxCursor, InboxPage, ListInbox};

/// Explicit limits refuse oversized traversal rather than print partial success.
const MAX_ROWS: usize = 10_000;
const MAX_PAGES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ReadError {
    #[error(
        "pending requests exceed this command's collection budget; use the inbox pages instead"
    )]
    Limit,
    #[error("the node returned a nonadvancing inbox cursor")]
    Cursor,
    #[error(transparent)]
    Unsupported(#[from] UnsupportedKind),
    #[error("the node returned the same pending request more than once")]
    Duplicate,
    #[error("pending requests have not been completely read")]
    Incomplete,
    #[error("the pending-request traversal has already finished")]
    Finished,
}

#[derive(Debug, Default)]
enum Phase {
    #[default]
    First,
    More(InboxCursor),
    Complete,
    Failed(ReadError),
}

/// The listing has no effects; mutations separately recheck the exact source key.
#[derive(Debug, Default)]
pub(crate) struct Pages {
    phase: Phase,
    rows: Vec<Row>,
    seen: HashSet<(SessionId, PendingId)>,
    pages: usize,
}

impl Pages {
    /// The only next page allowed by the current cursor, or explicit completion.
    pub(crate) fn request(&self) -> Result<Option<ListInbox>, ReadError> {
        match &self.phase {
            Phase::First => Ok(Some(ListInbox::default())),
            Phase::More(before) => Ok(Some(ListInbox {
                before: Some(*before),
            })),
            Phase::Complete => Ok(None),
            Phase::Failed(error) => Err(error.clone()),
        }
    }

    /// Validate cursor and identities before treating another page as progress.
    pub(crate) fn accept(&mut self, page: InboxPage) -> Result<(), ReadError> {
        let before = match self.request()? {
            Some(request) => request.before,
            None => return Err(ReadError::Finished),
        };
        let result = self.append(before, page);
        if let Err(error) = &result {
            self.phase = Phase::Failed(error.clone());
        }
        result
    }

    fn append(&mut self, before: Option<InboxCursor>, page: InboxPage) -> Result<(), ReadError> {
        if self.pages >= MAX_PAGES {
            return Err(ReadError::Limit);
        }
        self.pages = self.pages.saturating_add(1);
        if let (Some(before), Some(next)) = (before, page.next)
            && next >= before
        {
            return Err(ReadError::Cursor);
        }
        for entry in page.entries {
            if let Some(row) = Row::of(&entry)? {
                if self.rows.len() >= MAX_ROWS {
                    return Err(ReadError::Limit);
                }
                if !self.seen.insert((row.agent_id, row.id)) {
                    return Err(ReadError::Duplicate);
                }
                self.rows.push(row);
            }
        }
        self.phase = page.next.map_or(Phase::Complete, Phase::More);
        Ok(())
    }

    /// A failed or unfinished traversal cannot be formatted as a complete result.
    pub(crate) fn finish(self) -> Result<Vec<Row>, ReadError> {
        match self.phase {
            Phase::Complete => Ok(self.rows),
            Phase::Failed(error) => Err(error),
            Phase::First | Phase::More(_) => Err(ReadError::Incomplete),
        }
    }
}

#[cfg(test)]
mod tests;
