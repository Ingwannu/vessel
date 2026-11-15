//! What was typed in a composer and not sent, kept per session and for the
//! new chat across launches (Paseo's `stores/draft-store`, the page's
//! `data/drafts.ts` and the bridge's `keep/drafts_doc.rs`): the text comes
//! back when the composer does.
//!
//! [Decision Log]
//! - Purpose: the coordinator's row e (drafts kept per session), as Paseo
//!   keeps a draft per agent and the page per session.
//! - Constraint: the app writes its own file (no bridge); a draft can be
//!   typed in a session long gone, so the file must not grow without end.
//! - Alternatives: memory only (lost on quit; Paseo and the page both keep
//!   them on disk); one file per draft (many small writes, no total bound).
//! - Choice: one document of every draft, bounded as the bridge bounds the
//!   page's: at most [`Drafts::MAX_DRAFTS`] drafts, none older than
//!   [`Drafts::MAX_AGE_DAYS`] days, each at most [`Drafts::MAX_TEXT`] bytes,
//!   the oldest dropped first when the total passes
//!   [`Drafts::MAX_TOTAL`]. A recalled message shown in the composer is not
//!   a draft until it is edited (the page's `show`).
//! - Cost: a draft longer than [`Drafts::MAX_TEXT`] is kept for this
//!   launch only; the screen says so.

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::SessionId;

/// Whose composer a draft is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DraftKey {
    /// The new chat's.
    New,
    /// An independent draft tab in one canonical workspace on one node.
    Workspace {
        /// Authenticated node identity; an alias cannot redirect this draft.
        node: vessel_proto::NodeId,
        /// Canonical workspace registry identity, never a cwd-derived id.
        workspace: vessel_proto::WorkspaceId,
        /// The pane owner's existing draft identity, not a new allocation here.
        draft: crate::layout::DraftId,
    },
    /// A session's (session ids are unique across machines).
    Session {
        /// The session.
        session: SessionId,
    },
}

/// One kept draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    /// Whose.
    pub key: DraftKey,
    /// What was typed.
    pub text: String,
    /// When it was last typed in.
    pub at: DateTime<Utc>,
}

/// What became of a draft the composer handed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    /// Kept, to be written.
    Kept,
    /// Emptied: no draft for the key any more.
    Removed,
    /// Longer than a draft may be: in memory for this launch only.
    TooLong,
}

/// Every kept draft, oldest typed in first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drafts {
    drafts: Vec<Draft>,
}

impl Drafts {
    /// The most drafts kept (the bridge's `MAX_DRAFTS`).
    pub const MAX_DRAFTS: usize = 64;
    /// How old a kept draft may grow, in days (the bridge's `MAX_AGE`).
    pub const MAX_AGE_DAYS: i64 = 30;
    /// The longest draft kept, in bytes (the page's `MAX_DRAFT_BYTES`).
    pub const MAX_TEXT: usize = 128 * 1024;
    /// The most bytes of text kept in all (the bridge's `MAX_FILE`).
    pub const MAX_TOTAL: usize = 1024 * 1024;

    /// The draft of `key`, or nothing.
    #[must_use]
    pub fn text(&self, key: DraftKey) -> &str {
        self.drafts
            .iter()
            .find(|draft| draft.key == key)
            .map_or("", |draft| draft.text.as_str())
    }

    /// Keep `text` as the draft of `key`, typed at `now`; empty text (or
    /// only spaces) removes it.
    pub fn keep(&mut self, key: DraftKey, text: &str, now: DateTime<Utc>) -> Kept {
        self.drafts.retain(|draft| draft.key != key);
        if text.trim().is_empty() {
            return Kept::Removed;
        }
        if text.len() > Self::MAX_TEXT {
            return Kept::TooLong;
        }
        self.drafts.push(Draft {
            key,
            text: text.to_owned(),
            at: now,
        });
        self.prune(now);
        Kept::Kept
    }

    /// Drop what is too old, then the oldest until the count and the total
    /// fit (also run on a document read from disk, which may break them).
    pub fn prune(&mut self, now: DateTime<Utc>) {
        let oldest = now
            .checked_sub_signed(TimeDelta::days(Self::MAX_AGE_DAYS))
            .unwrap_or(DateTime::<Utc>::MIN_UTC);
        self.drafts
            .retain(|draft| draft.at >= oldest && draft.text.len() <= Self::MAX_TEXT);
        self.drafts.sort_by_key(|draft| draft.at);
        let mut total: usize = self.drafts.iter().map(|draft| draft.text.len()).sum();
        while self.drafts.len() > Self::MAX_DRAFTS || total > Self::MAX_TOTAL {
            if self.drafts.is_empty() {
                break;
            }
            let dropped = self.drafts.remove(0);
            total = total.saturating_sub(dropped.text.len());
        }
    }

    /// How many drafts are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.drafts.len()
    }

    /// Whether none is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.drafts.is_empty()
    }
}

#[cfg(test)]
mod tests;
