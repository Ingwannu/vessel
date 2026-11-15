//! The existing editor engine's literal search exposed without a second
//! matching implementation or a component-owned search box/theme.

use super::CodeEditor;
use gpui::{App, Window};

/// Literal search case policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FindCase {
    /// Reference default: ignore case.
    #[default]
    Insensitive,
    /// Match case exactly.
    Sensitive,
}

/// Search movement wraps in the engine's stable match order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindDirection {
    /// Next occurrence.
    Next,
    /// Previous occurrence.
    Previous,
}

/// Replacement is explicit; a readonly editor refuses either choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceScope {
    /// Replace one occurrence and advance.
    Current,
    /// Replace every occurrence in one undoable operation.
    All,
}

/// A count of literal matches, separate from bytes or characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchCount(pub usize);

/// Native search activation revision; reopening focuses/selects its query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindRevision(pub u64);

/// The engine's search UI state, independent of the caller's layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindState {
    /// No find UI should be mounted.
    Closed,
    /// An active search; `current` is one-based when there is a match.
    Open {
        /// Reinvoking the shortcut advances this even with the same query.
        revision: FindRevision,
        /// Seeded from the editor's selection when find opens.
        query: String,
        /// Current occurrence, independent of total matches.
        current: Option<std::num::NonZeroUsize>,
        /// All matches currently known by the local engine.
        total: MatchCount,
    },
}

impl CodeEditor {
    /// Snapshot the existing search session without rescanning the document.
    #[must_use]
    pub fn find_state(&self, cx: &App) -> FindState {
        let state = self.state.read(cx);
        let session = state.search_session();
        if !session.open {
            return FindState::Closed;
        }
        FindState::Open {
            revision: FindRevision(state.search_activation_revision()),
            query: session.query.clone(),
            current: session
                .matcher
                .current()
                .and_then(|index| index.checked_add(1))
                .and_then(std::num::NonZeroUsize::new),
            total: MatchCount(session.matcher.len()),
        }
    }

    /// The find button and the editor's native shortcut share one session.
    pub fn open_find(&self, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.open_search(false, cx));
    }

    /// End highlights and return focus to this document's editor.
    pub fn close_find(&self, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| state.close_search(cx));
        self.focus(window, cx);
    }

    /// Update a literal query and its case policy in the engine.
    pub fn find_query(&self, query: &str, case: FindCase, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.set_search_query(query, case == FindCase::Insensitive, cx)
        });
    }

    /// Move between highlighted matches and reveal the selected occurrence.
    pub fn find_move(&self, direction: FindDirection, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            let _range = match direction {
                FindDirection::Next => state.next_search_match(cx),
                FindDirection::Previous => state.previous_search_match(cx),
            };
        });
    }

    /// Replace matches; the caller must notify its document model when
    /// this returns a nonzero count because the engine's operation is silent.
    #[must_use]
    pub fn find_replace(
        &self,
        text: &str,
        scope: ReplaceScope,
        window: &mut Window,
        cx: &mut App,
    ) -> MatchCount {
        self.state.update(cx, |state, cx| {
            MatchCount(match scope {
                ReplaceScope::Current => {
                    usize::from(state.replace_current_search_match(text, window, cx))
                }
                ReplaceScope::All => state.replace_all_search_matches(text, window, cx),
            })
        })
    }
}
