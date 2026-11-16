//! Keep the ordinary lexer, but do not feed deleted text into added text state.
//!
//! [Decision Log]
//! - Existing asset: the source editor's `LineSyntax` and incremental cache.
//! - Problem: a unified diff contains two histories. A deleted block-comment
//!   opener must not colour a new, uncommented line as an old-side comment.
//! - Choice: retain two states of that same lexer, advancing only the source
//!   side present in each immutable row. Context advances both; headers reset
//!   the unknown gap, and fillers/notices never pretend to be source text.
//! - Cost: context is lexed twice when building the cache, never during paint.
//!   Sparse hunks cannot reconstruct syntax before their supplied context; no
//!   full-file syntax equivalence is claimed for unavailable earlier text.
use super::{DiffLine, DiffSide, DiffTone};
use crate::code_editor::{LineSyntax, Runs};
use std::sync::Arc;

pub(super) struct DiffSyntax<S> {
    lexer: S,
    rows: Arc<[DiffLine]>,
    side: DiffSide,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct State<T> {
    row: usize,
    original: T,
    modified: T,
}
impl<S: LineSyntax> DiffSyntax<S> {
    pub(super) const fn new(lexer: S, rows: Arc<[DiffLine]>, side: DiffSide) -> Self {
        Self { lexer, rows, side }
    }
}
impl<S: LineSyntax> LineSyntax for DiffSyntax<S> {
    type State = State<S::State>;
    fn language(&self) -> gpui::SharedString {
        self.lexer.language()
    }
    fn line(&self, text: &str, mut state: Self::State) -> (Runs, Self::State) {
        let kind = self
            .rows
            .get(state.row)
            .map_or(DiffTone::Empty, |row| row.tone);
        // An exhausted row cursor must never wrap back to another source row.
        state.row = state.row.saturating_add(1);
        let runs = match kind {
            DiffTone::Added => {
                let (runs, next) = self.lexer.line(text, state.modified);
                state.modified = next;
                runs
            }
            DiffTone::Removed => {
                let (runs, next) = self.lexer.line(text, state.original);
                state.original = next;
                runs
            }
            DiffTone::Context => {
                let (original, old) = self.lexer.line(text, state.original);
                let (modified, new) = self.lexer.line(text, state.modified);
                state.original = old;
                state.modified = new;
                match self.side {
                    DiffSide::Original => original,
                    DiffSide::Modified => modified,
                }
            }
            DiffTone::Header => {
                state.original = S::State::default();
                state.modified = S::State::default();
                Vec::new()
            }
            DiffTone::Empty | DiffTone::Notice => Vec::new(),
        };
        (runs, state)
    }
}

#[cfg(test)]
mod tests;
