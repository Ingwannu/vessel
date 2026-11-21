//! The text an agent reads for a `context` or `review` block (ADR 0124):
//! a context's own text (Paseo's `text` attachment), a review in Paseo's
//! form (`server/agent/prompt-attachments.ts` 91-117) with Vessel's name in
//! its heading.

use std::num::NonZeroU32;

use super::{PromptReview, ReviewLineKind};
use crate::ContentBlock;

/// What the agent reads for `block`: a context's text or a review's
/// rendering; `None` for every other block.
#[must_use]
pub fn agent_text(block: &ContentBlock) -> Option<String> {
    match block {
        ContentBlock::Context(context) => Some(context.text.as_str().to_owned()),
        ContentBlock::Review(review) => Some(review_text(review)),
        ContentBlock::Text { .. }
        | ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. } => None,
    }
}

/// A review as Paseo's server writes it: its mode, folder and base, then
/// each comment's place, words, hunk header and lines, the commented one
/// marked `> `.
pub(super) fn review_text(review: &PromptReview) -> String {
    let mut lines = vec![
        format!("Vessel review attachment ({})", review.mode.as_str()),
        format!("CWD: {}", review.root),
    ];
    if let Some(base) = review
        .base
        .as_ref()
        .filter(|base| !base.as_str().is_empty())
    {
        lines.push(format!("Base: {base}"));
    }
    for (index, comment) in review.comments.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!(
            "Comment {}: {}:{}:{}",
            index.saturating_add(1),
            comment.path,
            comment.side.as_str(),
            comment.line
        ));
        lines.push(comment.body.as_str().to_owned());
        lines.push(comment.hunk_header.as_str().to_owned());
        for line in &comment.lines {
            let prefix = if *line == comment.target { "> " } else { "  " };
            lines.push(format!(
                "{prefix}{} {} {}{}",
                number(line.old),
                number(line.new),
                marker(line.kind),
                line.content
            ));
        }
    }
    lines.join("\n")
}

/// A line number padded to two places, `-` for none (Paseo's
/// `padLineNumber`).
fn number(value: Option<NonZeroU32>) -> String {
    let text = value.map_or_else(|| "-".to_owned(), |number| number.to_string());
    format!("{text:>2}")
}

/// Paseo's `REVIEW_LINE_MARKERS`.
const fn marker(kind: ReviewLineKind) -> char {
    match kind {
        ReviewLineKind::Add => '+',
        ReviewLineKind::Remove => '-',
        ReviewLineKind::Context => ' ',
    }
}
