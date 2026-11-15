//! Syntax colour for the code editor from a tokenizer the app gives a line
//! at a time (the kit does not depend on `view`, where the tokenizer
//! lives): the engine's highlighter seam (`InputHighlighter`) fed by
//! [`LineSyntax`].
//!
//! [Decision Log]
//! - Purpose: the file being edited coloured as the read-only view colours
//!   it, by the same tokenizer, so editing a file does not change its look.
//! - Constraint: the engine asks for coloured runs over byte ranges and
//!   tells of each edit; a 1 MiB file has tens of thousands of lines, and
//!   a keystroke must not tokenize them all again.
//! - Alternatives: tree-sitter grammars (the component layer's; a parser
//!   per language and a second set of colours beside the view's lexer);
//!   tokenizing the whole text on every edit (simple, slow on large files).
//! - Choice: each line keeps the state it starts from, the state it leaves
//!   and its runs. An edit tokenizes from its first line through its last;
//!   past it, the first line whose start state is unchanged ends the work,
//!   and the rest is kept as it was.
//! - Colours: a line's runs carry kinds ([`SyntaxKind`]), not colours;
//!   the engine asks its style resolver for each kind's colour when it
//!   draws, and the editor projects the resolver from the current theme's
//!   code colours on every draw, so a theme change recolours at once.
//! - Cost: a block comment opened at the top of a large file still colours
//!   every line after it again, as it must.

use std::ops::Range;
use std::rc::Rc;

use gpui::{Context, HighlightStyle, SharedString, Window};
use gpui_base::input::{
    EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, Rope, RopeExt as _,
};

use crate::CodeTokens;

/// What a run of code is, for its colour (the view's token kinds, which
/// the app maps onto these).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxKind {
    /// A keyword.
    Keyword,
    /// A string.
    String,
    /// A comment.
    Comment,
    /// A number.
    Number,
    /// A constant (`true`, `None`).
    Constant,
    /// A called name, or markup's attribute.
    Function,
    /// Markup's tag (drawn as a keyword, as the page draws it).
    Tag,
}

impl SyntaxKind {
    /// The name the engine's style resolver is asked for.
    const fn name(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::String => "string",
            Self::Comment => "comment",
            Self::Number => "number",
            Self::Constant => "constant",
            Self::Function => "function",
            Self::Tag => "tag",
        }
    }
}

/// The current theme's code colours, as the engine's style resolver.
pub(super) struct CodeStyles(pub(super) CodeTokens);

impl HighlightStyleResolver for CodeStyles {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        let colours = &self.0;
        let colour = match name {
            "keyword" | "tag" => colours.keyword,
            "string" => colours.string,
            "comment" => colours.comment,
            "number" => colours.number,
            "constant" => colours.constant,
            "function" => colours.function,
            _ => return None,
        };
        Some(HighlightStyle {
            color: Some(colour),
            ..HighlightStyle::default()
        })
    }
}

/// A line's coloured runs: byte ranges of the line, each with its kind
/// (`None`: plain text).
pub type Runs = Vec<(Range<usize>, Option<SyntaxKind>)>;

/// A language's tokenizer, a line at a time.
pub trait LineSyntax: 'static {
    /// What a line leaves open for the next (inside a block comment, …).
    type State: Copy + Eq + Default + 'static;

    /// The language's name, as the engine names highlighters.
    fn language(&self) -> SharedString;

    /// The kind of each run of `text`'s bytes (`None`: plain), in order,
    /// and the state the line leaves. `text` has no line break.
    fn line(&self, text: &str, state: Self::State) -> (Runs, Self::State);
}

/// One line as last tokenized.
struct Line<S> {
    start: S,
    end: S,
    runs: Runs,
}

/// The engine's highlighter over a [`LineSyntax`].
pub(super) struct LineHighlighter<T: LineSyntax> {
    syntax: Rc<T>,
    lines: Vec<Line<T::State>>,
    /// Where each line starts, in bytes of the whole text.
    starts: Vec<usize>,
}

impl<T: LineSyntax> LineHighlighter<T> {
    pub(super) const fn new(syntax: Rc<T>) -> Self {
        Self {
            syntax,
            lines: Vec::new(),
            starts: Vec::new(),
        }
    }

    /// Tokenize `text` from line `first`, reusing what follows line
    /// `last` once a line starts in the state it started in before.
    fn retokenize(&mut self, text: &Rope, first: usize, last: usize) {
        let count = text.lines_len();
        let mut old: Vec<Option<Line<T::State>>> = std::mem::take(&mut self.lines)
            .into_iter()
            .map(Some)
            .collect();
        let old_len = old.len();
        let mut lines: Vec<Line<T::State>> = Vec::with_capacity(count);
        let kept = first.min(old_len).min(count);
        lines.extend(old.iter_mut().take(kept).filter_map(Option::take));
        let mut state = lines.last().map_or_else(T::State::default, |line| line.end);
        let mut row = lines.len();
        while row < count {
            // Past the edit, the line that was at `row + old_len - count`.
            let before = (row > last)
                .then(|| row.checked_add(old_len)?.checked_sub(count))
                .flatten();
            if let Some(before) = before
                && old
                    .get(before)
                    .and_then(Option::as_ref)
                    .is_some_and(|line| line.start == state)
            {
                lines.extend(old.iter_mut().skip(before).filter_map(Option::take));
                break;
            }
            let (runs, end) = self.syntax.line(&line_text(text, row), state);
            lines.push(Line {
                start: state,
                end,
                runs,
            });
            state = end;
            row = row.saturating_add(1);
        }
        lines.truncate(count);
        self.lines = lines;
        self.starts = (0..count).map(|row| text.line_start_offset(row)).collect();
    }
}

/// Line `row` of `text` without its line break.
fn line_text(text: &Rope, row: usize) -> String {
    // The engine's line slice keeps a `\r` (a CRLF file); the lexer sees
    // the line without it.
    let mut line = text.slice_line(row).to_string();
    while line.ends_with(['\n', '\r']) {
        let _was = line.pop();
    }
    line
}

impl<T: LineSyntax> InputHighlighter for LineHighlighter<T> {
    fn language(&self) -> SharedString {
        self.syntax.language()
    }

    fn update(
        &mut self,
        edit: Option<InputEdit>,
        text: &Rope,
        _folding: bool,
        _window: &mut Window,
        _cx: &mut Context<'_, EditorState>,
    ) {
        match edit {
            Some(edit) if !self.lines.is_empty() => {
                self.retokenize(text, edit.start_position.row, edit.new_end_position.row);
            }
            Some(_) | None => self.retokenize(text, 0, usize::MAX),
        }
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut out = Vec::new();
        let mut at = range.start;
        let first = self
            .starts
            .partition_point(|start| *start <= range.start)
            .saturating_sub(1);
        for (start, line) in self.starts.iter().zip(&self.lines).skip(first) {
            if *start >= range.end {
                break;
            }
            for (run, kind) in &line.runs {
                let from = start.saturating_add(run.start).max(at);
                let to = start.saturating_add(run.end).min(range.end);
                if from >= to {
                    continue;
                }
                if from > at {
                    out.push((at..from, HighlightStyle::default()));
                }
                let style = kind
                    .and_then(|kind| resolver.style(kind.name()))
                    .unwrap_or_default();
                out.push((from..to, style));
                at = to;
            }
        }
        if at < range.end {
            out.push((at..range.end, HighlightStyle::default()));
        }
        out
    }

    /// No folding: the panel's editor turns it off.
    fn fold_ranges(&self, _text: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::SharedString;
    use gpui_base::input::Rope;

    use super::{LineHighlighter, LineSyntax, Runs, SyntaxKind};

    /// Digits coloured; `/*` opens a comment that `*/` closes.
    struct Digits;

    impl LineSyntax for Digits {
        type State = bool;

        fn language(&self) -> SharedString {
            "digits".into()
        }

        fn line(&self, text: &str, state: bool) -> (Runs, bool) {
            let mut open = state;
            let mut runs = Vec::new();
            for (at, character) in text.char_indices() {
                if text.get(at..).is_some_and(|rest| rest.starts_with("/*")) {
                    open = true;
                }
                if character.is_ascii_digit() && !open {
                    runs.push((at..at.saturating_add(1), Some(SyntaxKind::Number)));
                }
                if text.get(..=at).is_some_and(|head| head.ends_with("*/")) {
                    open = false;
                }
            }
            (runs, open)
        }
    }

    #[test]
    fn colours_runs_across_lines_and_reuses_what_an_edit_did_not_reach() {
        let mut highlighter = LineHighlighter::new(Rc::new(Digits));
        let text = Rope::from_str("a1\nb\n2c\n");
        highlighter.retokenize(&text, 0, usize::MAX);
        assert_eq!(highlighter.lines.len(), 4);
        assert_eq!(highlighter.starts, vec![0, 3, 5, 8]);
        // A comment opened on the first line reaches every line after it.
        let edited = Rope::from_str("/*1\nb\n2c\n");
        highlighter.retokenize(&edited, 0, 0);
        assert!(highlighter.lines.iter().all(|line| line.runs.is_empty()));
        // Closed again: the lines after the edit are coloured as before.
        let closed = Rope::from_str("1\nb\n2c\n");
        highlighter.retokenize(&closed, 0, 0);
        assert_eq!(
            highlighter.lines.get(2).map(|line| line.runs.len()),
            Some(1)
        );
    }
}
