//! Review comments written on the Changes diff, kept until they are sent
//! (ADR 0124; Paseo's `review/state.ts` and `review/store.ts`): one list
//! of comments per review key, each on one line of one side of a file.
//!
//! [Decision Log]
//! - Purpose: the person comments lines of the diff, then sends the
//!   comments to an agent as one review (P6b-2). The comments must outlive
//!   a switch of session, of comparison, or a restart, as Paseo's do.
//! - Constraint: the drafts document is the app's one file of the
//!   person's unsent words (owner-only, written a second after the last
//!   change), and it is bounded; a review must not let it grow without
//!   end. A comment's body goes to the node as ADR 0124's `ReviewBody`
//!   (at most 16 KiB).
//! - Alternatives: memory only (lost on quit, Paseo keeps them on the
//!   device); a file of their own (a second owner-only file, its own
//!   debounce and set-aside rules for the same kind of text).
//! - Choice: a field of the drafts document, read and written with it;
//!   Paseo's key (machine, root, mode, base, whitespace), its comment
//!   shape and its reducers (add, edit with a new `updated_at`, delete,
//!   clear, clear what was sent by id, `updated_at` and body). Bounds:
//!   [`ReviewDrafts::MAX_COMMENTS`] comments a key and
//!   [`ReviewDrafts::MAX_KEYS`] keys, the oldest out first, and
//!   [`ReviewDrafts::MAX_BYTES`] bytes of bodies in all.
//! - Cost: a body longer than [`BODY_BYTES`] cannot be saved (Paseo has
//!   no bound); the editor's Comment stays off for it, as for a blank one.

use std::num::NonZeroU32;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use vessel_proto::prompt_context::{BODY_BYTES, ReviewMode, ReviewSide};
use vessel_proto::workspace::{RelPath, WorkspaceRoot};

pub mod feedback;
mod snapshot;
pub use snapshot::{Snapshot, snapshot};

/// The review a composer offers while the Changes diff shows comments
/// (Paseo's workspace review attachment): its key, to clear it once sent,
/// and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// The comments' key.
    pub key: ReviewKey,
    /// The review.
    pub snapshot: Snapshot,
}

/// Which review a comment belongs to: the diff it was written on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewKey {
    /// The machine-book name of the node.
    pub machine: String,
    /// The root the diff is of.
    pub root: WorkspaceRoot,
    /// The comparison.
    pub mode: ReviewMode,
    /// The base named by the comparison, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Whether white space was ignored (Paseo's key part; the native diff
    /// has no such option yet, so it is always false).
    #[serde(default)]
    pub ignore_whitespace: bool,
}

/// One line of one side of a file: where a comment sits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewTarget {
    /// The file, in the root.
    pub path: RelPath,
    /// The side: old for a removed line, new otherwise.
    pub side: ReviewSide,
    /// The line's number on that side.
    pub line: NonZeroU32,
}

/// A comment's identity, kept across edits: the microsecond it was
/// written, moved past every identity the document holds, so it is never
/// reused while a comment of that identity may still be on its way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(u64);

/// One comment not sent yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftComment {
    /// Its identity.
    pub id: CommentId,
    /// Where it sits.
    #[serde(flatten)]
    pub target: ReviewTarget,
    /// What it says, trimmed.
    pub body: String,
    /// When it was written.
    pub created_at: DateTime<Utc>,
    /// When it was last edited.
    pub updated_at: DateTime<Utc>,
}

/// What became of a comment handed to [`ReviewDrafts::save`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Saved {
    /// A new comment, with its identity.
    Added(CommentId),
    /// The comment of that identity, its body replaced.
    Edited(CommentId),
    /// Only white space: nothing saved.
    Blank,
    /// Longer than [`BODY_BYTES`]: nothing saved.
    TooLong,
}

/// Whether `body` may be saved: not blank, within [`BODY_BYTES`] once
/// trimmed.
#[must_use]
pub fn can_save(body: &str) -> bool {
    let body = body.trim();
    !body.is_empty() && body.len() <= BODY_BYTES
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Review {
    key: ReviewKey,
    comments: Vec<DraftComment>,
}

/// Every review not sent yet, the one changed longest ago first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReviewDrafts {
    reviews: Vec<Review>,
}

impl ReviewDrafts {
    /// The most comments kept for one key (the oldest go first).
    pub const MAX_COMMENTS: usize = 200;
    /// The most keys kept (the one changed longest ago goes first).
    pub const MAX_KEYS: usize = 64;
    /// The most bytes of bodies kept in all.
    pub const MAX_BYTES: usize = 1024 * 1024;

    /// Whether no comment is kept.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.reviews.is_empty()
    }

    /// The comments of `key`, oldest first.
    #[must_use]
    pub fn comments(&self, key: &ReviewKey) -> &[DraftComment] {
        self.reviews
            .iter()
            .find(|review| review.key == *key)
            .map_or(&[], |review| review.comments.as_slice())
    }

    /// The comments of `key` on `target`, oldest first.
    pub fn on(
        &self,
        key: &ReviewKey,
        target: &ReviewTarget,
    ) -> impl Iterator<Item = &DraftComment> + '_ {
        let target = target.clone();
        self.comments(key)
            .iter()
            .filter(move |comment| comment.target == target)
    }

    /// Paseo's `saveReviewDraftComment`: the body of comment `id` when it
    /// is still kept, else a new comment on `target`; at `now`.
    pub fn save(
        &mut self,
        key: &ReviewKey,
        id: Option<CommentId>,
        target: ReviewTarget,
        body: &str,
        now: DateTime<Utc>,
    ) -> Saved {
        let body = body.trim();
        if body.is_empty() {
            return Saved::Blank;
        }
        if body.len() > BODY_BYTES {
            return Saved::TooLong;
        }
        let mut review = self.take(key);
        let kept = id.and_then(|id| review.comments.iter_mut().find(|c| c.id == id));
        let saved = if let Some(comment) = kept {
            body.clone_into(&mut comment.body);
            comment.updated_at = now;
            Saved::Edited(comment.id)
        } else {
            let id = self.next_id(&review, now);
            review.comments.push(DraftComment {
                id,
                target,
                body: body.to_owned(),
                created_at: now,
                updated_at: now,
            });
            Saved::Added(id)
        };
        self.reviews.push(review);
        self.prune();
        saved
    }

    /// Remove comment `id` of `key`; whether it was kept.
    pub fn delete(&mut self, key: &ReviewKey, id: CommentId) -> bool {
        let Some(review) = self.reviews.iter_mut().find(|review| review.key == *key) else {
            return false;
        };
        let before = review.comments.len();
        review.comments.retain(|comment| comment.id != id);
        let deleted = review.comments.len() < before;
        self.reviews.retain(|review| !review.comments.is_empty());
        deleted
    }

    /// Remove every comment of `key` (a composer sent the review).
    pub fn clear(&mut self, key: &ReviewKey) {
        self.reviews.retain(|review| review.key != *key);
    }

    /// Remove the comments of `key` sent as `sent`: only those whose
    /// identity, `updated_at` and body are still the sent ones (an edit
    /// made while the review was on its way stays). How many went.
    pub fn clear_sent(&mut self, key: &ReviewKey, sent: &[DraftComment]) -> usize {
        let Some(review) = self.reviews.iter_mut().find(|review| review.key == *key) else {
            return 0;
        };
        let before = review.comments.len();
        review.comments.retain(|comment| {
            !sent.iter().any(|was| {
                was.id == comment.id
                    && was.updated_at == comment.updated_at
                    && was.body == comment.body
            })
        });
        let cleared = before.saturating_sub(review.comments.len());
        self.reviews.retain(|review| !review.comments.is_empty());
        cleared
    }

    /// Keep the bounds (also run on a document read from disk): no empty
    /// review, no blank or too long body, at most [`Self::MAX_COMMENTS`] a
    /// key, at most [`Self::MAX_KEYS`] keys and [`Self::MAX_BYTES`] bytes,
    /// the oldest out first.
    pub fn prune(&mut self) {
        for review in &mut self.reviews {
            review.comments.retain(|comment| can_save(&comment.body));
            let over = review.comments.len().saturating_sub(Self::MAX_COMMENTS);
            drop(review.comments.drain(..over));
        }
        self.reviews.retain(|review| !review.comments.is_empty());
        let over = self.reviews.len().saturating_sub(Self::MAX_KEYS);
        drop(self.reviews.drain(..over));
        let mut bytes: usize = self.reviews.iter().map(Review::bytes).sum();
        while bytes > Self::MAX_BYTES && !self.reviews.is_empty() {
            let dropped = self.reviews.remove(0);
            bytes = bytes.saturating_sub(dropped.bytes());
        }
    }

    /// An identity past every one kept (`review` is out of the list while
    /// it is changed), from `now`'s microsecond.
    fn next_id(&self, review: &Review, now: DateTime<Utc>) -> CommentId {
        let kept = self
            .reviews
            .iter()
            .chain(std::iter::once(review))
            .flat_map(|review| review.comments.iter().map(|comment| comment.id.0))
            .max()
            .map_or(0, |id| id.saturating_add(1));
        let at = u64::try_from(now.timestamp_micros()).unwrap_or(0);
        CommentId(at.max(kept))
    }

    /// The review of `key` taken out (a new one when none is kept), to be
    /// put back last: the one changed most recently.
    fn take(&mut self, key: &ReviewKey) -> Review {
        self.reviews
            .iter()
            .position(|review| review.key == *key)
            .map_or_else(
                || Review {
                    key: key.clone(),
                    comments: Vec::new(),
                },
                |at| self.reviews.remove(at),
            )
    }
}

impl Review {
    fn bytes(&self) -> usize {
        self.comments.iter().map(|comment| comment.body.len()).sum()
    }
}

#[cfg(test)]
mod tests;
