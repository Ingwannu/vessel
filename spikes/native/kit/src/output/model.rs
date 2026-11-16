//! Output data shared by the plain and annotated virtualized renderer.
use gpui::SharedString;
use std::ops::Range;

/// Where a text opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// At its top (a call's input, a diff).
    Top,
    /// At its end, following it while it grows (a command's output).
    End,
}

/// What a line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Text.
    Plain,
    /// A diff's added line.
    Added,
    /// A diff's removed line.
    Removed,
}

/// One line of a box.
#[derive(Debug, Clone)]
pub struct Line {
    /// Its text.
    pub text: SharedString,
    /// What it is.
    pub tone: Tone,
    /// Its number, in a diff.
    pub number: Option<usize>,
}

/// What a box shows: its lines and what its copy button copies.
#[derive(Debug, Clone)]
pub struct Lines {
    /// The lines.
    pub lines: Vec<Line>,
    /// The text put on the clipboard; no copy button without one.
    pub copy: Option<SharedString>,
}

impl Lines {
    /// `text`'s lines, plain, the whole text to copy.
    #[must_use]
    pub fn text(text: &str) -> Self {
        Self {
            lines: text
                .lines()
                .map(|line| Line {
                    text: line.to_owned().into(),
                    tone: Tone::Plain,
                    number: None,
                })
                .collect(),
            copy: Some(text.to_owned().into()),
        }
    }
}

/// Optional byte-range emphasis over the existing line/copy model.
/// Empty or invalid ranges preserve ordinary whole-line rendering.
#[derive(Debug, Clone)]
pub struct AnnotatedLines {
    /// Existing lines and exact copy payload.
    pub lines: Lines,
    /// Changed ranges by line, validated against that line at cache admission.
    pub emphasis: Vec<Vec<Range<usize>>>,
}

impl From<Lines> for AnnotatedLines {
    fn from(lines: Lines) -> Self {
        Self {
            lines,
            emphasis: Vec::new(),
        }
    }
}

/// Semantic syntax over the same virtualized lines and exact copy payload.
/// Kinds, not resolved colors, survive warm draws and theme changes.
#[derive(Debug, Clone)]
pub struct SyntaxLines {
    /// Existing text, numbering, tones and optional changed ranges.
    pub output: AnnotatedLines,
    /// Non-overlapping UTF-8 byte runs per displayed line.
    pub syntax: Vec<crate::code_editor::Runs>,
}

impl From<AnnotatedLines> for SyntaxLines {
    fn from(output: AnnotatedLines) -> Self {
        Self {
            output,
            syntax: Vec::new(),
        }
    }
}

impl From<Lines> for SyntaxLines {
    fn from(lines: Lines) -> Self {
        Self::from(AnnotatedLines::from(lines))
    }
}
