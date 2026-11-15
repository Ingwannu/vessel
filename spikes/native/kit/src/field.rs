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
mod send;
mod submit_press;
pub use send::SendOn;
pub use submit_press::SubmitPress;
mod editability;
mod events;
pub use editability::Editability;
use events::{Handlers, listen};
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

/// The most lines the composer grows to before it scrolls.
const COMPOSER_LINES: usize = 12;

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

/// GOAT's small textarea grows within its 256-point maximum. At the
/// kit's 20-point line height twelve complete lines fit within that cap.
const FORM_LINES: usize = 12;
/// Three lines plus GOAT's small insets and border fit its desktop minimum.
const FORM_MIN_LINES: usize = 3;
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
    composer_rows: Rc<Cell<Option<composer_layout::Rows>>>,
    submit_press: Rc<Cell<Option<SubmitPress>>>,
}

impl TextField {
    /// A form field showing `placeholder` while empty. Enter in it calls
    /// `on_submit` on its owner with the text it holds.
    pub fn new<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(placeholder, on_submit, None, window, cx)
    }

    /// The same, also calling `on_change` with the text after every edit
    /// (to say what is wrong with it beside the field as it is typed).
    pub fn with_change<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        on_change: fn(&mut V, String, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(placeholder, on_submit, Some(on_change), window, cx)
    }

    /// A composer's text: lines that grow; the press [`SendOn`] names
    /// (Enter unless [`Self::set_send_on`] says otherwise) calls
    /// `on_submit` with the text, Shift+Enter starts a new line.
    ///
    /// The engine sends on Enter and Mod+Enter and starts a line on
    /// Shift+Enter (`submit_on_enter`); in [`SendOn::ModEnter`] a plain
    /// Enter that reaches here starts a line instead (the engine's own
    /// Enter-starts-a-line mode would start one on Mod+Enter too, before
    /// sending).
    pub fn composer<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::multiline(MultilineStyle::Composer, placeholder, on_submit, window, cx)
    }

    /// A multiline form field on the one-line form field's surface.
    /// Enter starts a line; Mod+Enter submits. It grows from three to
    /// twelve lines, then the existing text engine scrolls. GOAT's small
    /// textarea supplies the 68 / 80-point minimum and 256-point cap.
    pub fn form_multiline<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::multiline(MultilineStyle::Form, placeholder, on_submit, window, cx)
    }

    fn multiline<V: 'static>(
        style: MultilineStyle,
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let placeholder = placeholder.to_owned();
        let minimum = match style {
            MultilineStyle::Composer => 1,
            MultilineStyle::Form => FORM_MIN_LINES,
        };
        let maximum = match style {
            MultilineStyle::Composer => COMPOSER_LINES,
            MultilineStyle::Form => FORM_LINES,
        };
        let state = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(minimum, maximum)
                .submit_on_enter(true)
                .placeholder(placeholder)
        });
        let focused = Rc::new(Cell::new(false));
        let submit_press = Rc::new(Cell::new(None));
        let shortcut = match style {
            MultilineStyle::Composer => SendOn::Enter,
            MultilineStyle::Form => SendOn::ModEnter,
        };
        let send = Rc::new(Cell::new(shortcut));
        listen(
            &state,
            Handlers {
                on_submit: Box::new(on_submit),
                submit_press: Rc::clone(&submit_press),
                // A growing form changes the surrounding dialog's layout
                // and submit availability; its owner must see each edit.
                on_change: match style {
                    MultilineStyle::Composer => None,
                    MultilineStyle::Form => Some(|_, _, cx| cx.notify()),
                },
                focused: Rc::clone(&focused),
                send: Some(Rc::clone(&send)),
            },
            window,
            cx,
        );
        register(Known::Lines(state.downgrade()));
        Self {
            composer_rows: Rc::new(Cell::new(None)),
            submit_press,
            kind: Kind::Lines(state, style),
            focused,
            send,
        }
    }

    fn line<V: 'static>(
        placeholder: &str,
        on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
        on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let placeholder = placeholder.to_owned();
        let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let focused = Rc::new(Cell::new(false));
        let submit_press = Rc::new(Cell::new(None));
        listen(
            &state,
            Handlers {
                on_submit: Box::new(on_submit),
                submit_press: Rc::clone(&submit_press),
                on_change,
                focused: Rc::clone(&focused),
                send: None,
            },
            window,
            cx,
        );
        register(Known::Line(state.downgrade()));
        Self {
            composer_rows: Rc::new(Cell::new(None)),
            submit_press,
            kind: Kind::Line(state),
            focused,
            send: Rc::new(Cell::new(SendOn::Enter)),
        }
    }

    /// What it holds.
    #[must_use]
    pub fn text(&self, cx: &App) -> String {
        match &self.kind {
            Kind::Line(state) => state.read(cx).value().to_string(),
            Kind::Lines(state, _) => state.read(cx).value().to_string(),
        }
    }

    /// Replace what it holds.
    pub fn set_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        let text = text.to_owned();
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| state.set_value(text, window, cx)),
            Kind::Lines(state, _) => {
                state.update(cx, |state, cx| state.set_value(text, window, cx));
            }
        }
    }

    /// Change what it shows while empty (the text table's words, chosen
    /// after the field is made).
    pub fn set_placeholder(&self, placeholder: &str, window: &mut Window, cx: &mut App) {
        let placeholder = placeholder.to_owned();
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| {
                state.set_placeholder(placeholder, window, cx);
            }),
            Kind::Lines(state, _) => state.update(cx, |state, cx| {
                state.set_placeholder(placeholder, window, cx);
            }),
        }
    }

    /// Empty it.
    pub fn clear(&self, window: &mut Window, cx: &mut App) {
        self.set_text("", window, cx);
    }

    /// Its focus handle: to hear it lose the keys (a title edited in
    /// place saves when the person leaves the field).
    #[must_use]
    pub fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        match &self.kind {
            Kind::Line(state) => state.read(cx).focus_handle(cx),
            Kind::Lines(state, _) => state.read(cx).focus_handle(cx),
        }
    }

    /// Whether this field holds the keys and an input method is composing
    /// in it (marked text, a Hangul syllable not yet committed): a
    /// shortcut must leave the keys to the input method then (S2's
    /// shortcut listener asks this of the focused field). Only the
    /// focused field answers yes: marked text left in a field without the
    /// keys says nothing about the keys being typed now.
    pub fn composing(&self, window: &mut Window, cx: &mut App) -> bool {
        use gpui::EntityInputHandler as _;

        if !self.focus_handle(cx).is_focused(window) {
            return false;
        }
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| {
                state.marked_text_range(window, cx).is_some()
            }),
            Kind::Lines(state, _) => state.update(cx, |state, cx| {
                state.marked_text_range(window, cx).is_some()
            }),
        }
    }

    /// Whether this field has the keys with its caret at the very start and
    /// nothing selected (track S2's Alt+Up, Paseo's edit-the-last-queued
    /// prompt, acts only there, so the key still moves the caret anywhere
    /// else). A field without the keys answers no.
    pub fn at_start(&self, window: &Window, cx: &App) -> bool {
        if !self.focus_handle(cx).is_focused(window) {
            return false;
        }
        let range = match &self.kind {
            Kind::Line(state) => state.read(cx).selected_range(),
            Kind::Lines(state, _) => state.read(cx).selected_range(),
        };
        range.start == 0 && range.end == 0
    }

    /// Whether any kit text field in this window has the keys and an input
    /// method composing in it: what a window-wide shortcut listener asks
    /// before acting on a key (track S2's registry), since the fields are
    /// private to the areas that own them.
    ///
    /// [Decision Log]
    /// - Purpose: S2's shortcuts must leave keys to an input method in
    ///   every field, not only the ones S2 knows about (the pairing form,
    ///   a conversation's find, queue and rename, Devices, Projects, and
    ///   whatever G1 and G3 add).
    /// - Constraint: those fields are private to their areas; a list of
    ///   them in S2's code would miss the next one.
    /// - Choice: every field the kit makes registers a weak handle to its
    ///   text engine (one per field, thread-local: GPUI's entities live on
    ///   the main thread); this asks each live one, dropping the dead.
    ///   Only the focused field counts, as in [`Self::composing`].
    /// - Cost: one weak handle per field ever made, pruned here; a field
    ///   made outside the kit (none today) is not seen.
    pub fn focused_composing(window: &mut Window, cx: &mut App) -> bool {
        use gpui::EntityInputHandler as _;

        let known = KNOWN.with(|known| std::mem::take(&mut *known.borrow_mut()));
        let mut alive = Vec::with_capacity(known.len());
        let mut composing = false;
        for field in known {
            let marked = match &field {
                Known::Line(weak) => weak.upgrade().map(|state| {
                    !composing
                        && state.read(cx).focus_handle(cx).is_focused(window)
                        && state.update(cx, |state, cx| {
                            state.marked_text_range(window, cx).is_some()
                        })
                }),
                Known::Lines(weak) => weak.upgrade().map(|state| {
                    !composing
                        && state.read(cx).focus_handle(cx).is_focused(window)
                        && state.update(cx, |state, cx| {
                            state.marked_text_range(window, cx).is_some()
                        })
                }),
            };
            if let Some(marked) = marked {
                composing |= marked;
                alive.push(field);
            }
        }
        KNOWN.with(|known| {
            // Fields made while asking (none can be, but keep them).
            let mut known = known.borrow_mut();
            alive.append(&mut known);
            *known = alive;
        });
        composing
    }

    /// Give it the keys.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        let handle = match &self.kind {
            Kind::Line(state) => state.read(cx).focus_handle(cx),
            Kind::Lines(state, _) => state.read(cx).focus_handle(cx),
        };
        handle.focus(window, cx);
    }
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
