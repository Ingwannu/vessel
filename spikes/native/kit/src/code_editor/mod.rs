//! A file's text being edited: the engine's code-editor kind (`gpui-base`'s
//! `EditorState`: rope, undo, selection, input methods, line numbers,
//! search) drawn in the kit's colours and type, coloured by the caller's
//! line tokenizer ([`LineSyntax`]). Paseo's source editor
//! (`file-panels/source/editor/view.web.tsx`, `CodeMirror`) is the model:
//! line numbers, the code face, no box and no focus ring.
//!
//! [Decision Log]
//! - Purpose: the code panel edits files in place (Paseo's Source panel).
//! - Constraint: an editing engine (selection, clipboard, undo, input
//!   methods, large text) is weeks of work; the engine the kit's fields
//!   already use has a code-editor kind with all of it.
//! - Alternatives: the kit's multi-line field (no line numbers, no syntax
//!   colour); the component layer's styled editor (its own box, ring and
//!   theme painted over ours on every frame, the owner's objection to the
//!   fields).
//! - Choice: the engine's unstyled editor; its colours projected from the
//!   kit's tokens on every draw (as `field.rs` does), the code face with
//!   Pretendard behind it (Hangul never falls to boxes or a bitmap face),
//!   folding off (Paseo's source panel has none), lines unwrapped unless
//!   the caller asks (Paseo wraps Markdown only), syntax from the caller.
//! - Cost: the engine's caret and selection drawing are its own (the
//!   caret stops blinking while motion is held, the engine's own rule).

mod diff;
mod syntax;

pub use diff::{
    DiffDocumentEditor, DiffEditor, DiffLine, DiffRowIndex, DiffSide, DiffText, DiffTextError,
    DiffTone, DiffWrapping,
};

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Entity, Window, div};
use gpui_base::input::{
    Editor, EditorState, InputEditorStyle, InputEvent, InputHighlighter, InputHighlighterFactory,
    Position, RopeExt as _,
};

pub use syntax::{LineSyntax, Runs, SyntaxKind};

use crate::theme::spacing;
use crate::{Tokens, Type, TypeExt as _};

/// The room before the line numbers: `CodeMirror`'s gutter element's left
/// padding, which Paseo keeps (not a step of the kit's spacing).
const GUTTER_LEAD: f32 = 5.0;

/// A file's text in the engine's code editor (a handle: clones share it).
#[derive(Clone)]
pub struct CodeEditor {
    state: Entity<EditorState>,
}

impl std::fmt::Debug for CodeEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeEditor").finish_non_exhaustive()
    }
}

impl CodeEditor {
    /// An editor over `text`, coloured by `syntax`; `on_change` is called
    /// after each change the person makes (not after [`Self::set_text`]).
    pub fn new<V: 'static, S: LineSyntax>(
        text: &str,
        syntax: S,
        on_change: fn(&mut V, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let language = syntax.language();
        let syntax = Rc::new(syntax);
        let text = text.to_owned();
        let state = cx.new(|cx| {
            // Code keeps each line on one row (the engine wraps unless
            // told otherwise); prose asks for wrapping (`set_wrapping`).
            let mut state = EditorState::new(window, cx)
                .folding(false)
                .line_number(true)
                .soft_wrap(false);
            // Paseo's `CodeMirror` theme: 16 above and below the text, and
            // `CodeMirror`'s 5 before the line numbers (without it they
            // touch the panel's edge once a file has four digits of lines).
            state.set_editor_paddings(gpui::Edges {
                top: gpui::px(spacing::S4),
                right: gpui::px(spacing::S0),
                bottom: gpui::px(spacing::S4),
                left: gpui::px(GUTTER_LEAD),
            });
            let factory: InputHighlighterFactory = Rc::new(move |_language| {
                let highlighter: Box<dyn InputHighlighter> =
                    Box::new(syntax::LineHighlighter::new(Rc::clone(&syntax)));
                Some(highlighter)
            });
            state.set_highlighter_factory(factory, cx);
            state.set_highlighter(language, cx);
            state.set_value(text, window, cx);
            state
        });
        cx.subscribe_in(
            &state,
            window,
            move |owner, _state, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    on_change(owner, window, cx);
                }
            },
        )
        .detach();
        Self { state }
    }

    /// The text as it is now.
    #[must_use]
    pub fn text(&self, cx: &App) -> String {
        self.state.read(cx).value().to_string()
    }

    /// Replace the whole text (a reload): no change is reported, and undo
    /// starts again from here.
    pub fn set_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        let text = text.to_owned();
        self.state
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }

    /// Where the caret is: line and column, both from 1 (Paseo's
    /// "Ln 3, Col 7").
    #[must_use]
    pub fn cursor(&self, cx: &App) -> (u32, u32) {
        let Position { line, character } = self.state.read(cx).cursor_position();
        (line.saturating_add(1), character.saturating_add(1))
    }

    /// The lines the selection covers, from 1, `None` with no selection.
    #[must_use]
    pub fn selected_lines(&self, cx: &App) -> Option<(u32, u32)> {
        let state = self.state.read(cx);
        let range = state.selected_range();
        if range.is_empty() {
            return None;
        }
        let text = state.text();
        let line_of = |byte: usize| {
            let row = text.offset_to_point(byte.min(text.len())).row;
            u32::try_from(row).unwrap_or(u32::MAX).saturating_add(1)
        };
        // A selection that ends at the start of a line does not take it.
        let end = range.end.saturating_sub(1).max(range.start);
        Some((line_of(range.start), line_of(end)))
    }

    /// Put the caret at the start of `line` (from 1), scrolled into view.
    pub fn reveal_line(&self, line: u32, window: &mut Window, cx: &mut App) {
        let position = Position {
            line: line.saturating_sub(1),
            character: 0,
        };
        self.state.update(cx, |state, cx| {
            state.set_cursor_position(position, window, cx);
        });
    }

    /// Wrap long lines at the editor's width (prose), or keep each line on
    /// one row, scrolled sideways (code, the default: Paseo wraps only
    /// Markdown).
    pub fn set_wrapping(&self, wrap: bool, window: &mut Window, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.set_soft_wrap(wrap, window, cx));
    }

    /// Show the text without letting it be changed (a phone's width, a file
    /// over the editable size); selection and copy still work.
    pub fn set_read_only(&self, read_only: bool, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.set_readonly(read_only, cx));
    }

    /// Follow every change of the editor, the caret's moves and blinks
    /// included: for a small view of its own (the caret's line and column,
    /// what the selection allows), never for a whole screen, which would
    /// be drawn again twice a second while nothing else moves.
    pub fn observe<T: 'static>(
        &self,
        cx: &mut Context<'_, T>,
        on_change: fn(&mut T, &mut Context<'_, T>),
    ) -> gpui::Subscription {
        cx.observe(&self.state, move |this, _state, cx| on_change(this, cx))
    }

    /// Give the editor the keys.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| state.focus(window, cx));
    }

    /// The editor, filling its parent, in the code face.
    pub fn element(&self, tokens: &Tokens, cx: &mut App) -> AnyElement {
        project(&self.state, tokens, cx);
        div()
            .size_full()
            .type_(Type::Mono)
            .child(Editor::new(&self.state))
            .into_any_element()
    }
}

/// The kit's colours on the engine: the foreground's text and caret, the
/// muted foreground for the line numbers, a selection of the foreground at
/// a fifth (as the fields), the panel's surface behind the text and the
/// gutter, no fill on the current line, and the theme's code colours for
/// the syntax kinds.
fn project(state: &Entity<EditorState>, tokens: &Tokens, cx: &mut App) {
    state.update(cx, |state, _cx| state.set_editor_style(style(tokens)));
}

/// Keep the ordinary editor and read-only diff on the same syntax/theme projection.
pub(super) fn style(tokens: &Tokens) -> InputEditorStyle {
    let neutral = tokens.neutral;
    InputEditorStyle {
        foreground: neutral.foreground,
        muted_foreground: neutral.foreground_muted,
        background: neutral.surface0,
        border: neutral.border,
        caret: neutral.foreground,
        selection: neutral.foreground.opacity(0.2),
        // Paseo's current line has no fill, only its number in the
        // foreground.
        editor_active_line: None,
        editor_gutter_background: Some(neutral.surface0),
        highlight_styles: std::sync::Arc::new(syntax::CodeStyles(crate::CodeTokens::of(
            tokens.scheme,
        ))),
        ..InputEditorStyle::default()
    }
}
