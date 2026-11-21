//! The review sent to an agent, from the kept comments and the diff now
//! shown (Paseo's `buildReviewAttachmentSnapshot`, `review/store.ts`):
//! each comment's line found in its file's hunk, with up to three lines
//! either side; a comment whose line left the diff is dropped.
//!
//! [Decision Log]
//! - Purpose: "Send feedback" and the composer's review pill send ADR
//!   0124's `PromptReview`, built from what the person sees.
//! - Constraint: the native diff is read a file at a time, as its rows come
//!   into view; a file not read yet has no rows. The wire bounds a review
//!   (100 comments, 7 lines each, 64 KiB of text) and each value (a diff
//!   line at most 4 KiB, no line break).
//! - Choice: Paseo's matching over the rows read (`rows_of`); a comment on
//!   a file not read is left out until it is read (the caller asks for
//!   it). Lines a value refuses are left out of the context; a comment
//!   whose target or body the wire refuses is left out; comments past
//!   the review's bounds are left out, in order. The comments taken are
//!   returned with the review, so only those are cleared once it is sent.
//! - Cost: a review of more than the wire's bounds sends what fits, and
//!   its count says so ("Send feedback (N)" counts what is sent).

use std::num::NonZeroU32;
use std::str::FromStr as _;

use vessel_proto::ContentBlock;
use vessel_proto::prompt_context::{
    MAX_COMMENTS, PromptReview, ReviewComment, ReviewLine, ReviewLineKind, ReviewText,
};
use vessel_proto::workspace::{LineKind, RelPath};

use super::{DraftComment, ReviewKey, ReviewSide};
use crate::workspace::diff_rows::DiffRow;

/// Lines of context either side of a comment's line (Paseo's
/// `CONTEXT_RADIUS`).
const RADIUS: usize = 3;

/// A review ready to send, and the comments it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// What the agent is sent.
    pub review: PromptReview,
    /// The kept comments it was built from (cleared once it is sent).
    pub sent: Vec<DraftComment>,
}

impl Snapshot {
    /// What an agent before protocol 46 reads instead: the same text.
    #[must_use]
    pub fn text(&self) -> String {
        vessel_proto::prompt_context::agent_text(&ContentBlock::Review(self.review.clone()))
            .unwrap_or_default()
    }
}

/// The review of `comments` (kept under `key`) over the diff rows read,
/// rooted at `root` (the folder's path the agent works in); none when no
/// comment's line is in the diff.
pub fn snapshot<'a>(
    root: &str,
    key: &ReviewKey,
    comments: &[DraftComment],
    rows_of: impl Fn(&RelPath) -> Option<&'a [DiffRow]>,
) -> Option<Snapshot> {
    let mut review = PromptReview {
        root: ReviewText::from_str(root).ok()?,
        mode: key.mode,
        base: key
            .base
            .as_deref()
            .and_then(|base| ReviewText::from_str(base).ok()),
        comments: Vec::new(),
    };
    let mut sent = Vec::new();
    for comment in comments {
        if review.comments.len() >= MAX_COMMENTS {
            break;
        }
        let Some(found) = rows_of(&comment.target.path).and_then(|rows| comment_of(comment, rows))
        else {
            continue;
        };
        review.comments.push(found);
        if review.check().is_err() {
            drop(review.comments.pop());
            continue;
        }
        sent.push(comment.clone());
    }
    (!review.comments.is_empty()).then_some(Snapshot { review, sent })
}

/// `comment` as the wire's, found in `rows`.
fn comment_of(comment: &DraftComment, rows: &[DiffRow]) -> Option<ReviewComment> {
    let line = comment.target.line.get();
    let at = rows.iter().position(|row| match row {
        DiffRow::Line { old, new, .. } => match comment.target.side {
            ReviewSide::Old => *old == Some(line),
            ReviewSide::New => *new == Some(line),
        },
        DiffRow::Hunk { .. } | DiffRow::Cut { .. } => false,
    })?;
    let start = rows
        .get(..at)?
        .iter()
        .rposition(|row| matches!(row, DiffRow::Hunk { .. }))?;
    let header = match rows.get(start)? {
        DiffRow::Hunk { header, section, .. } => section
            .as_ref()
            .map_or_else(|| header.clone(), |section| format!("{header} {section}")),
        DiffRow::Line { .. } | DiffRow::Cut { .. } => return None,
    };
    let after = rows.get(at..)?;
    let end = after
        .iter()
        .position(|row| !matches!(row, DiffRow::Line { .. }))
        .map_or(rows.len(), |gap| at.saturating_add(gap));
    let hunk = rows.get(start.saturating_add(1)..end)?;
    let place = at.saturating_sub(start.saturating_add(1));
    let from = place.saturating_sub(RADIUS);
    let to = place.saturating_add(RADIUS).saturating_add(1).min(hunk.len());
    let lines = hunk.get(from..to)?.iter().filter_map(line_of).collect();
    Some(ReviewComment {
        path: comment.target.path.clone(),
        side: comment.target.side,
        line: comment.target.line,
        body: comment.body.parse().ok()?,
        hunk_header: ReviewText::from_str(&header).ok()?,
        target: line_of(rows.get(at)?)?,
        lines,
    })
}

/// A diff row as the wire's line; none for a header, a cut, "No newline",
/// or text the wire refuses.
fn line_of(row: &DiffRow) -> Option<ReviewLine> {
    let DiffRow::Line {
        line,
        text,
        old,
        new,
        ..
    } = row
    else {
        return None;
    };
    let kind = match line {
        LineKind::Added => ReviewLineKind::Add,
        LineKind::Removed => ReviewLineKind::Remove,
        LineKind::Context => ReviewLineKind::Context,
        LineKind::NoNewline => return None,
    };
    Some(ReviewLine {
        old: old.and_then(NonZeroU32::new),
        new: new.and_then(NonZeroU32::new),
        kind,
        content: ReviewText::from_str(text.trim_end_matches('\r')).ok()?,
    })
}
