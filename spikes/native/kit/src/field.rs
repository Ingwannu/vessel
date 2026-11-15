//! Text editing state shared by every kit presentation: one factory, the
//! weak composing-field registry and focus queries. Engine events stay shared
//! in `events`; editability comes from its sole shared child module.
//! Presentation and color projection live in the child modules; they do not
//! create another editor or subscribe to its events.
//!
//! [Decision Log]
//! - Purpose: leave room for the reserved rename/select-all and composer
//!   extensions without growing the listed field module.
//! - Choice: move existing presentation and theme projection unchanged;
//!   keep factories, line registration, IME and composer hooks unchanged.
//! - Constraint: one Entity remains the editing state on every surface.
//!   Public entry points and all existing event behavior stay unchanged.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, Context, Entity, Focusable as _, Window};
#[cfg(test)]
use gpui::{div, px};
use gpui_base::input::{InputState, TextareaState};

pub mod composer_hooks;
pub mod composer_layout;
mod multiline;
pub use multiline::SendOn;
mod form_minimum;
pub use form_minimum::FormMinimum;
mod send;
mod submit_press;
pub use submit_press::SubmitPress;
mod editability;
mod events;
pub use editability::Editability;
use events::{Callbacks, Handlers, listen};
mod identity;
pub use identity::FieldId;
mod frame;
mod presentation;
mod rename;
mod selection;
mod styles;

pub(crate) use frame::settings_frame;
pub use presentation::composer_card;
pub(crate) use styles::apply_tokens;

#[cfg(test)]
use crate::theme::Tokens;

/// The composer's text's smallest height (Paseo's `MIN_INPUT_HEIGHT`).
const COMPOSER_MIN: f32 = 46.0;

/// What a field edits.
#[derive(Debug, Clone)]
enum Kind {
    /// One line.
    Line(Entity<InputState>),
    /// Lines that grow (the composer).
    Lines(Entity<TextareaState>, MultilineStyle),
}

/// A field's text engine, held weakly for [`TextField::focused_composing`].
enum Known {
    Line(gpui::WeakEntity<InputState>),
    Lines(gpui::WeakEntity<TextareaState>),
}

thread_local! {
    /// Every field the kit made on this thread (GPUI's main thread), live
    /// or not yet pruned.
    static KNOWN: std::cell::RefCell<Vec<Known>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn register(field: Known) {
    KNOWN.with(|known| known.borrow_mut().push(field));
}

/// A multiline editor's shell; both presentations share one text engine.
#[derive(Debug, Clone, Copy)]
enum MultilineStyle {
    Composer,
    Form,
}

/// GOAT's small control: 66 / 78 points inside its one-point border.
const FORM_MIN: f32 = 68.0;
const FORM_MIN_TOUCH: f32 = 80.0;
const FORM_MAX: f32 = 256.0;
/// GOAT's small insets: spacing(2.5) - 1 and spacing(1) - 1.
const FORM_INSET_X: f32 = 9.0;
const FORM_INSET_Y: f32 = 3.0;

/// A text field: a form's one line, or a composer's text.
#[derive(Debug, Clone)]
pub struct TextField {
    kind: Kind,
    /// Whether it holds the keys, as its focus and blur events say (its
    /// border shows it).
    focused: Rc<Cell<bool>>,
    /// Which press sends (a composer's; a form field's Enter always
    /// submits).
    send: Rc<Cell<SendOn>>,
    form_minimum: FormMinimum,
    composer_rows: Rc<Cell<Option<composer_layout::Rows>>>,
    submit_press: Rc<Cell<Option<SubmitPress>>>,
}

#[cfg(test)]
mod tests {
    use super::SendOn;

    #[test]
    fn enter_sends_unless_the_shortcut_is_mod_enter() {
        assert!(SendOn::Enter.sends(false));
        assert!(SendOn::Enter.sends(true));
        assert!(!SendOn::ModEnter.sends(false));
        assert!(SendOn::ModEnter.sends(true));
    }
}

#[cfg(test)]
mod composing_tests {
    use gpui::{EntityInputHandler as _, Render, TestAppContext};

    use super::*;

    struct Probe {
        line: TextField,
        lines: TextField,
        form: TextField,
    }

    /// The fields' submit (nothing to do in this test).
    fn submit(_: &mut Probe, _: String, _: &mut Window, _: &mut Context<'_, Probe>) {}

    impl Render for Probe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            let tokens = Tokens::of(crate::theme::Scheme::Dark);
            div()
                .child(self.line.element(&tokens, false, cx))
                .child(self.lines.element(&tokens, false, cx))
                .child(self.form.element(&tokens, false, cx))
        }
    }

    /// Marked text in `field` (an input method's syllable in progress).
    fn mark(field: &TextField, text: &str, window: &mut Window, cx: &mut App) {
        match &field.kind {
            Kind::Line(state) => state.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, text, None, window, cx);
            }),
            Kind::Lines(state, _) => state.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, text, None, window, cx);
            }),
        }
    }

    fn unmark(field: &TextField, window: &mut Window, cx: &mut App) {
        match &field.kind {
            Kind::Line(state) => state.update(cx, |state, cx| state.unmark_text(window, cx)),
            Kind::Lines(state, _) => state.update(cx, |state, cx| state.unmark_text(window, cx)),
        }
    }

    /// Composing is the focused field's marked text only: marked text in a
    /// field without the keys does not count, and committing ends it
    /// (S2's request through V1, 2026-11-13).
    #[gpui::test]
    fn only_the_focused_field_composes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx, crate::SchemeChoice::Fixed(crate::theme::Scheme::Dark))
                .expect("the faces register");
        });
        let (probe, cx) = cx.add_window_view(|window, cx| Probe {
            line: TextField::new("", submit, window, cx),
            lines: TextField::composer("", submit, window, cx),
            form: TextField::form_multiline("", submit, window, cx),
        });
        cx.run_until_parked();
        probe.update_in(cx, |probe, window, cx| {
            for (focused, other) in [
                (&probe.line, &probe.lines),
                (&probe.lines, &probe.form),
                (&probe.form, &probe.line),
            ] {
                focused.focus(window, cx);
                assert!(!focused.composing(window, cx), "nothing marked yet");
                mark(other, "ㅎ", window, cx);
                assert!(!other.composing(window, cx), "the other field has no keys");
                assert!(!focused.composing(window, cx), "the other's marked text");
                assert!(
                    !TextField::focused_composing(window, cx),
                    "only the other field is marked"
                );
                mark(focused, "하", window, cx);
                assert!(focused.composing(window, cx), "a syllable in progress");
                assert!(
                    TextField::focused_composing(window, cx),
                    "the window's focused field"
                );
                unmark(focused, window, cx);
                assert!(!focused.composing(window, cx), "committed");
                assert!(!TextField::focused_composing(window, cx), "committed");
                // The caret after the committed syllable is not at the
                // start; emptied, it is, and only for the field with keys.
                assert!(!focused.at_start(window, cx), "after 하");
                focused.clear(window, cx);
                assert!(focused.at_start(window, cx), "empty, focused");
                assert!(!other.at_start(window, cx), "no keys");
                unmark(other, window, cx);
            }
        });
    }
}

#[cfg(test)]
mod form_tests;
#[cfg(test)]
mod submit_tests;
