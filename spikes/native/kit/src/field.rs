//! The text field: one line of text a person types, input methods
//! included (Hangul is composed by the system's input method and
//! committed through GPUI's input handler).
//!
//! Under it is `gpui-component`'s input, the one part of that library the
//! kit uses: writing a text field with selection, clipboard, undo and
//! input-method composition is weeks of work, and theirs is used in a
//! shipping app. Screens see only [`TextField`], so it
//! can be replaced without touching them.

use gpui::{App, AppContext as _, Context, Entity, Focusable as _, IntoElement, Window};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::fonts;
use crate::theme::Tokens;

/// A text field.
#[derive(Debug, Clone)]
pub struct TextField {
    state: Entity<InputState>,
}

impl TextField {
    /// A field showing `placeholder` while empty. Enter in it calls
    /// `on_submit` on its owner with the text it holds.
    pub fn new<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let placeholder = placeholder.to_owned();
        let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        cx.subscribe_in(
            &state,
            window,
            move |owner, state, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => {
                    let text = state.read(cx).value().to_string();
                    on_submit(owner, text, window, cx);
                }
                InputEvent::Change | InputEvent::Focus | InputEvent::Blur => {}
            },
        )
        .detach();
        Self { state }
    }

    /// What it holds.
    #[must_use]
    pub fn text(&self, cx: &App) -> String {
        self.state.read(cx).value().to_string()
    }

    /// Empty it.
    pub fn clear(&self, window: &mut Window, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.set_value("", window, cx));
    }

    /// Give it the keyboard.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        let handle = self.state.read(cx).focus_handle(cx);
        handle.focus(window, cx);
    }

    /// The field, drawn.
    #[must_use]
    pub fn element(&self) -> impl IntoElement {
        Input::new(&self.state)
    }
}

/// The component layer's colours and type from the kit's tokens, so its
/// input looks like the page's (DESIGN.md: `--bg-surface`, a subtle
/// hairline, the ink's caret, the ring on focus, 14px Geist).
pub(crate) fn apply_tokens(cx: &mut App, tokens: &Tokens) {
    let theme = gpui_component::Theme::global_mut(cx);
    theme.font_family = fonts::SANS.into();
    theme.mono_font_family = fonts::MONO.into();
    theme.font_size = gpui::px(14.0);
    theme.radius = gpui::px(crate::theme::radius::R2);
    let colors = &mut theme.colors;
    colors.background = tokens.bg_base;
    colors.foreground = tokens.text_primary;
    colors.border = tokens.border_subtle;
    colors.input = tokens.border_subtle;
    colors.ring = tokens.ring;
    colors.caret = tokens.text_primary;
    colors.muted_foreground = tokens.text_tertiary;
    colors.selection = tokens.bg_active;
    colors.popover = tokens.bg_elevated;
    colors.popover_foreground = tokens.text_primary;
    colors.accent = tokens.bg_hover;
    colors.accent_foreground = tokens.text_primary;
}
