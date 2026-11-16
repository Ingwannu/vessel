//! File/directory mention detection and correlated suggestion state.
//! The input engine uses UTF-8 byte offsets; the same visible token rules
//! as Paseo are applied before a path lookup is scheduled.

mod state;
pub use state::{Failure, Menu, Phase, Request, Scope, Suggestion, SuggestionKind, Ticket};
use std::ops::Range;
use vessel_proto::workspace::{FindQuery, InvalidText, RelPath};

/// Paseo waits for typing to settle before requesting directory suggestions.
pub const DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(180);
/// A cached result can be reused without asking the same node again.
pub const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(15);
/// The composer's reference result count, independent of the node's ceiling.
pub const RESULT_LIMIT: usize = 50;
/// Native cache bound; expired or least recently used queries leave first.
pub const CACHE_QUERIES: usize = 32;

/// Empty mentions list children; nonempty ones reuse the existing validated query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// List the selected root's children.
    Children,
    /// Search paths with the node's existing bounded query type.
    Find(FindQuery),
}
impl Query {
    /// Plain query text for cache keys and list selection.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Children => "",
            Self::Find(query) => query.as_str(),
        }
    }
}

/// Why a visible mention could not become a safe query.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// Retain the validator's source instead of parsing an error string.
    #[error(transparent)]
    Query(#[from] InvalidText),
    /// Correlation must never wrap onto a previous request.
    #[error("file mention correlation identities exhausted")]
    Exhausted,
}

/// The token replaced by one quoted path; offsets always lie on UTF-8 boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    range: Range<usize>,
    query: Query,
}
impl Mention {
    /// The exact input span, including its leading `@`.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    /// The validated lookup, including blank-query child listing.
    #[must_use]
    pub fn query(&self) -> &Query {
        &self.query
    }
}

/// Detect the nearest eligible `@`, preserving email and Unicode behavior.
///
/// # Errors
/// The active query violates the node's existing `FindQuery` bounds.
pub fn active(text: &str, caret: usize) -> Result<Option<Mention>, Problem> {
    let mut end = caret.min(text.len());
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let before = text.get(..end).unwrap_or_default();
    for (start, ch) in before.char_indices().rev() {
        if ch != '@' {
            continue;
        }
        if before
            .get(..start)
            .and_then(|prefix| prefix.chars().next_back())
            .is_some_and(|prev| prev.is_ascii_alphanumeric() || prev == '_' || prev == '+')
        {
            continue;
        }
        let query = before
            .get(start.saturating_add('@'.len_utf8())..)
            .unwrap_or_default();
        if query
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '<' | '>'))
        {
            continue;
        }
        let query = if query.is_empty() {
            Query::Children
        } else {
            Query::Find(query.parse()?)
        };
        return Ok(Some(Mention {
            range: start..end,
            query,
        }));
    }
    Ok(None)
}

/// A choice replaces the mention with a quoted relative path, never `@path`.
#[must_use]
pub fn quoted(path: &RelPath) -> String {
    format!("\"{}\"", path.as_str().replace('"', "\\\""))
}

#[cfg(test)]
mod tests;
