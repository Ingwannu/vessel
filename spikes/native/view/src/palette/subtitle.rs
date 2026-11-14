//! Semantic captions preserve a complete searchable sentence while the
//! renderer gives its path the code face. Splitting search fields would
//! change reference offsets, relevance and cross-part token matches.

use std::borrow::Cow;
use std::ops::Range;

use crate::matcher::Query;

/// A node's displayed path, not a path interpreted by this client's OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayPath(String);

impl DisplayPath {
    /// Preserve node spelling, including paths from another operating system.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The original display spelling, before layout truncation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A palette subtitle, before typography and matching marks are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subtitle {
    /// A machine's connection state, in the interface face.
    Plain(String),
    /// A session status and a path, in distinct interface/code elements.
    StatusPath {
        /// The localized complete state phrase.
        status: String,
        /// The node's directory or its explicit display fallback.
        path: DisplayPath,
    },
}

/// Original text with ranges measured in this part's own UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPart {
    /// Complete text; the drawing control owns truncation.
    pub text: String,
    /// All visible matches, including subsequence runs.
    pub marks: Vec<Range<usize>>,
}

/// Marked semantic parts, without a drawing toolkit or font metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkedSubtitle {
    /// One plain caption.
    Plain(TextPart),
    /// A prose prefix (including its separator) followed by a code path.
    StatusPath {
        /// Status and separator; separator matches remain visible.
        prefix: TextPart,
        /// Path marks start at the beginning of the path element.
        path: TextPart,
    },
}

impl Subtitle {
    /// Keep the exact pre-split visible sentence as one search field.
    #[must_use]
    pub fn search_text(&self) -> Cow<'_, str> {
        match self {
            Self::Plain(text) => Cow::Borrowed(text),
            Self::StatusPath { status, path } => {
                Cow::Owned(format!("{status} · {}", path.as_str()))
            }
        }
    }

    /// Match the complete displayed sentence once, then intersect byte
    /// ranges with each semantic part. Never re-match pieces independently.
    #[must_use]
    pub fn marked(&self, query: &Query) -> MarkedSubtitle {
        let whole = self.search_text();
        let marks = query.marks(&whole);
        match self {
            Self::Plain(_) => MarkedSubtitle::Plain(TextPart {
                text: whole.into_owned(),
                marks,
            }),
            Self::StatusPath { status, path } => {
                let prefix = format!("{status} · ");
                let boundary = prefix.len();
                MarkedSubtitle::StatusPath {
                    prefix: TextPart {
                        text: prefix,
                        marks: project(&marks, 0..boundary),
                    },
                    path: TextPart {
                        text: path.as_str().to_owned(),
                        marks: project(&marks, boundary..whole.len()),
                    },
                }
            }
        }
    }
}

fn project(marks: &[Range<usize>], part: Range<usize>) -> Vec<Range<usize>> {
    marks
        .iter()
        .filter_map(|mark| {
            let start = mark.start.max(part.start);
            let end = mark.end.min(part.end);
            (start < end)
                .then_some(start.saturating_sub(part.start)..end.saturating_sub(part.start))
        })
        .collect()
}
