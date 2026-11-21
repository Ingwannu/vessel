//! Context and review a person attaches to a prompt (ADR 0124, drafted as
//! protocol 46): a pull request's comment, review or check logs, a
//! workspace file and the like as one titled [`PromptContext`], and the
//! comments of a diff review as one [`PromptReview`].
//!
//! [Decision Log]
//! - Purpose: Paseo sends these with a prompt as typed attachments; its
//!   server renders them into the agent's text, and its timeline keeps
//!   them as pills (`prompt-attachments.ts` 40-117). Vessel's clients sent
//!   them as plain text blocks, so the timeline showed every word of a
//!   check's logs in the person's message.
//! - Constraint: an agent reads text; a peer decodes only the blocks of
//!   its version; a block's words come from forges and reviewers, and a
//!   prompt also waits in the queue and is kept in the log.
//! - Alternatives: text rendered by each client (no wire change, the whole
//!   text in the timeline, a renderer per client).
//! - Choice: two blocks in what a client sends. Each value checks its own
//!   bounds as it decodes; a block's whole bounds are checked where a
//!   prompt enters the node ([`check`]). The agent receives
//!   [`agent_text`]; a peer before [`PROMPT_CONTEXT_PROTOCOL`] receives
//!   that text as a `text` block (`older.rs`).
//! - Cost: a review's text is rendered again for each older peer that
//!   reads it, and once more when it is sent.

use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::ContentBlock;
use crate::key::serde_via_text;
use crate::node::ProtocolVersion;
use crate::workspace::RelPath;

mod older;
mod render;
#[cfg(test)]
mod tests;

pub(crate) use older::{block_for_protocol, content_protocol};
pub use render::agent_text;

/// The `context` and `review` prompt blocks (ADR 0124). Drafted as 46; the
/// integrator assigns the number at the node-line merge.
pub const PROMPT_CONTEXT_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(46));

/// The longest title or subtitle, in characters.
pub const TITLE_CHARS: usize = 200;
/// The most bytes a context's text, or a review rendered for the agent,
/// takes.
pub const BLOCK_BYTES: usize = 64 * 1024;
/// The most bytes of one review comment's words.
pub const BODY_BYTES: usize = 16 * 1024;
/// The most comments one review carries.
pub const MAX_COMMENTS: usize = 100;
/// The most diff lines around a comment's line (Paseo's three either side
/// and the line itself).
pub const MAX_LINES: usize = 7;
/// The longest line of a diff, a hunk header or a review's root, in bytes.
const LINE_BYTES: usize = 4 * 1024;
/// The longest page address, in bytes.
const URL_BYTES: usize = 2 * 1024;

/// What is wrong with a context or review value, or a whole block.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContextProblem {
    /// A value broke its rule.
    #[error("not a valid {kind}: {rule}")]
    Value {
        /// What it was meant to be.
        kind: &'static str,
        /// The rule it broke.
        rule: &'static str,
    },
    /// A review with no comment, or more than [`MAX_COMMENTS`].
    #[error("a review carries 1 to 100 comments; this one {0}")]
    Comments(usize),
    /// A comment with more diff lines than [`MAX_LINES`].
    #[error("a review comment carries at most 7 diff lines; this one {0}")]
    Lines(usize),
    /// A review whose text for the agent passes [`BLOCK_BYTES`].
    #[error("a review's text is at most 64 KiB; this one {0} bytes")]
    TooLarge(usize),
}

const fn invalid(kind: &'static str, rule: &'static str) -> ContextProblem {
    ContextProblem::Value { kind, rule }
}

/// Declares a checked string: `FromStr` by `$rule`, serde through it.
macro_rules! context_text {
    ($(#[$meta:meta])* $name:ident, $kind:literal, $rule:path) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);

        impl $name {
            /// The text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = ContextProblem;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                $rule(text)
                    .map(|()| Self(Arc::from(text)))
                    .map_err(|rule| invalid($kind, rule))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        serde_via_text!($name);
    };
}

context_text!(
    /// A pill's title or subtitle: one line of 1 to [`TITLE_CHARS`]
    /// characters.
    ContextTitle,
    "context title",
    title_rule
);
context_text!(
    /// What the agent reads of a context: at most [`BLOCK_BYTES`] bytes,
    /// not blank, no NUL.
    ContextText,
    "context text",
    text_rule
);
context_text!(
    /// A context's page: `https` or `http`, at most 2 KiB, no white space.
    ContextUrl,
    "context address",
    url_rule
);
context_text!(
    /// A reviewer's words on one line: at most [`BODY_BYTES`] bytes, not
    /// blank, no NUL.
    ReviewBody,
    "review comment",
    body_rule
);
context_text!(
    /// One line of a diff, a hunk header, a review's root or base: no line
    /// break or NUL, at most 4 KiB.
    ReviewText,
    "review line",
    line_rule
);

fn title_rule(text: &str) -> Result<(), &'static str> {
    if text.trim().is_empty() {
        return Err("not blank");
    }
    if text.chars().count() > TITLE_CHARS {
        return Err("at most 200 characters");
    }
    if text.chars().any(char::is_control) {
        return Err("one line, no control character");
    }
    Ok(())
}

fn text_rule(text: &str) -> Result<(), &'static str> {
    if text.trim().is_empty() {
        return Err("not blank");
    }
    if text.len() > BLOCK_BYTES {
        return Err("at most 64 KiB");
    }
    if text.contains('\0') {
        return Err("no NUL");
    }
    Ok(())
}

fn url_rule(text: &str) -> Result<(), &'static str> {
    if text.len() > URL_BYTES {
        return Err("at most 2 KiB");
    }
    if text
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err("no white space or control character");
    }
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
        .ok_or("https or http")?;
    if rest.is_empty() || rest.starts_with('/') {
        return Err("a host");
    }
    Ok(())
}

fn body_rule(text: &str) -> Result<(), &'static str> {
    if text.trim().is_empty() {
        return Err("not blank");
    }
    if text.len() > BODY_BYTES {
        return Err("at most 16 KiB");
    }
    if text.contains('\0') {
        return Err("no NUL");
    }
    Ok(())
}

fn line_rule(text: &str) -> Result<(), &'static str> {
    if text.len() > LINE_BYTES {
        return Err("at most 4 KiB");
    }
    if text.contains(['\n', '\r', '\0']) {
        return Err("no line break or NUL");
    }
    Ok(())
}

/// What a context is (Paseo's attachment kinds), for its pill's mark and
/// words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    /// A pull request's comment, or a whole review thread.
    PullComment,
    /// A pull request's review.
    PullReview,
    /// A pull request's check and its logs.
    PullCheck,
    /// A file of the workspace, named by its path.
    WorkspaceFile,
    /// An element picked in the browser.
    BrowserElement,
    /// Earlier conversation carried into a new one.
    ChatHistory,
    /// A forge's issue.
    ForgeIssue,
    /// A forge's pull request.
    ForgeChangeRequest,
    /// A plugin's resource.
    PluginResource,
}

/// One piece of context sent with a prompt (Paseo's titled `text`
/// attachment): the agent reads `text`; a timeline draws the pill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptContext {
    /// What it is.
    pub kind: ContextKind,
    /// The pill's first line.
    pub title: ContextTitle,
    /// The pill's second line, when the kind's word is not enough (a
    /// file's path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<ContextTitle>,
    /// What the agent reads.
    pub text: ContextText,
    /// Its page, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<ContextUrl>,
}

/// Which changes a review's comments were made on: the Changes pane's
/// scope (`workspace::DiffScope`'s kinds; Paseo's `uncommitted`, `base`
/// and `all`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    /// The working tree against the index.
    Unstaged,
    /// The index against `HEAD`.
    Staged,
    /// The working tree against `HEAD`.
    Uncommitted,
    /// The working tree against where it forked from a base.
    Base,
    /// One commit against its first parent.
    Commit,
}

impl ReviewMode {
    /// The mode as the agent's text names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unstaged => "unstaged",
            Self::Staged => "staged",
            Self::Uncommitted => "uncommitted",
            Self::Base => "base",
            Self::Commit => "commit",
        }
    }
}

/// Which side of the diff a comment is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ReviewSide {
    /// The line as it was.
    Old,
    /// The line as it is.
    New,
}

impl ReviewSide {
    /// The side as the agent's text names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Old => "old",
            Self::New => "new",
        }
    }
}

/// What a diff line does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ReviewLineKind {
    /// Added.
    Add,
    /// Removed.
    Remove,
    /// Unchanged, shown around a change.
    Context,
}

/// One line of the diff around a comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReviewLine {
    /// Its number on the old side; absent for an added line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<NonZeroU32>,
    /// Its number on the new side; absent for a removed line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<NonZeroU32>,
    /// What it does.
    pub kind: ReviewLineKind,
    /// Its text, without the diff's marker.
    pub content: ReviewText,
}

/// One comment on a line of the diff, with the lines around it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReviewComment {
    /// The file, under the review's root.
    pub path: RelPath,
    /// The side its line is on.
    pub side: ReviewSide,
    /// The line's number on that side.
    pub line: NonZeroU32,
    /// What the reviewer wrote.
    pub body: ReviewBody,
    /// The hunk's header (`@@ -a,b +c,d @@ …`).
    pub hunk_header: ReviewText,
    /// The commented line.
    pub target: ReviewLine,
    /// The lines around it within its hunk, the commented one included, at
    /// most [`MAX_LINES`].
    pub lines: Vec<ReviewLine>,
}

/// A diff review sent with a prompt (Paseo's `review` attachment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptReview {
    /// The workspace's folder, as the agent's text names it.
    pub root: ReviewText,
    /// Which changes the comments were made on.
    pub mode: ReviewMode,
    /// The base or commit those changes are against, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<ReviewText>,
    /// The comments, in the order the agent reads them.
    pub comments: Vec<ReviewComment>,
}

impl PromptReview {
    /// Whether the review keeps its bounds as a whole: 1 to
    /// [`MAX_COMMENTS`] comments, each with at most [`MAX_LINES`] lines,
    /// and its text for the agent within [`BLOCK_BYTES`].
    ///
    /// # Errors
    /// The first bound it breaks.
    pub fn check(&self) -> Result<(), ContextProblem> {
        let count = self.comments.len();
        if count == 0 || count > MAX_COMMENTS {
            return Err(ContextProblem::Comments(count));
        }
        if let Some(comment) = self
            .comments
            .iter()
            .find(|comment| comment.lines.len() > MAX_LINES)
        {
            return Err(ContextProblem::Lines(comment.lines.len()));
        }
        let bytes = render::review_text(self).len();
        if bytes > BLOCK_BYTES {
            return Err(ContextProblem::TooLarge(bytes));
        }
        Ok(())
    }
}

/// Whether each `context` and `review` block of `content` keeps its bounds
/// as a whole (a value's own bounds were checked as it decoded).
///
/// # Errors
/// The first bound a block breaks.
pub fn check(content: &[ContentBlock]) -> Result<(), ContextProblem> {
    content.iter().try_for_each(|block| match block {
        ContentBlock::Review(review) => review.check(),
        ContentBlock::Text { .. }
        | ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. }
        | ContentBlock::Context(_) => Ok(()),
    })
}
