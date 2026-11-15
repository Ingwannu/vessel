//! Text a person types: form fields and the composer's text
//! (lines that grow), input methods included (Hangul is composed by the
//! system's input method and committed through GPUI's input handler).
//!
//! [Decision Log]
//! - Purpose: the owner, twice, on 0.0.0.0.3: no library's default look
//!   ("입력칸같은거 내가 기본 그거 쓰지 말랬지 … paseo 처럼 그냥 입력되게
//!   해줘 이상한 강조 저거 기본형 놔두지말고"): Paseo's, where the composer's
//!   card is the input and nothing inside it has a box or a ring.
//! - Constraint: writing an editing engine (selection, clipboard, undo,
//!   input-method composition) is weeks of work. The component layer's
//!   styled input draws its own box and focus ring, and repaints its theme
//!   over a field's colours on every frame.
//! - Alternatives: the styled input with its look switched off
//!   (`appearance(false)`: still one placeholder colour for every field,
//!   the theme's); a field of our own over GPUI's input handler.
//! - Choice: the engine's unstyled inputs (`gpui-base`: `Input`,
//!   `Textarea`), which draw only text, caret and selection; everything
//!   around them is drawn here from Paseo's values. Each field projects its
//!   own colours (text, placeholder, caret, selection) when drawn.
//! - The look, Paseo's: a form field (`form-field.tsx`,
//!   `control-geometry.ts`) is `surface2`, 8 round, 12 across, 32 tall (44
//!   on a phone), 14 on 20; its border is clear at rest and `borderAccent`
//!   on hover and while it holds the keys. The composer's text
//!   (`composer/input/input.tsx`) has no box at all: 15 on 21, the
//!   foreground's colour, `surface4` for its placeholder, at least 46
//!   tall, growing a line at a time; Enter sends and Shift+Enter starts a
//!   line (the engine's `submit_on_enter`).
//! - Deliberate difference: Paseo's form field also draws a 2-point accent
//!   outline while focused (`:focus-visible`); the owner objected to the
//!   emphasis on a field being typed into, so a Vessel field shows focus
//!   by its border alone. The composer grows to 12 lines, then scrolls
//!   (Paseo: half the window).
//!
//! [Decision Log: multiline forms]
//! - Purpose: commit messages and pull-request bodies need a form field,
//!   rather than the composer's chat card.
//! - Constraints: preserve one editing state, input-method handler and
//!   border-only focus treatment; keep existing callers unchanged.
//! - Alternatives: the composer card, a styled library textarea, or a
//!   second editor with separate composition and keyboard handling.
//! - Choice: two typed multiline presentations over the same factory;
//!   the form shares the one-line field's shell. GOAT's small textarea
//!   supplies its 68/80 minimum, 9/3 insets and 256 maximum.
//! - Why: input methods, selection and undo cannot diverge between the
//!   composer and a form; one-line form chrome also stays in one place.
//! - Cost: the native form caps at twelve whole 20-point lines within
//!   that maximum, rather than GOAT's pixel-based cap. Enter starts a
//!   line and Mod+Enter submits. The existing engine owns scrolling.
//! - Boundary: engine wiring is shared through `events`, color projection
//!   through `styles`; these presentations share both. Existing layout
//!   and marked-text regressions stay in the parent's `form_tests` module.

use gpui::prelude::*;
use gpui::{AnyElement, App, div, px};
use gpui_base::input::{Input, Textarea};

use super::{COMPOSER_MIN, FORM_INSET_X, FORM_INSET_Y, FORM_MAX, Kind, MultilineStyle, TextField};
use super::{form_minimum::form_shell, styles::project};
use crate::fonts::{self, Type};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

impl TextField {
    /// A one-line field's input alone, without its shell: for a field
    /// inside a control of its own (the find bar's search, with its
    /// count and buttons in the same shell). A composer's text has no
    /// such use and draws nothing here.
    #[must_use]
    pub fn input(&self, tokens: &Tokens, cx: &mut App) -> AnyElement {
        match &self.kind {
            Kind::Line(state) => {
                project(state, tokens, tokens.neutral.foreground_muted, cx);
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .type_(Type::Body)
                    .line_height(px(fonts::ui(20.0)))
                    .text_color(tokens.neutral.foreground)
                    .child(Input::new(state))
                    .into_any_element()
            }
            Kind::Lines(_, _) => div().into_any_element(),
        }
    }

    /// The field, drawn: a form field's chrome around a line, or a
    /// composer's bare text (the card around it is the screen's,
    /// [`composer_card`]). `narrow`: a phone's width (44-point fields).
    pub fn element(&self, tokens: &Tokens, narrow: bool, cx: &mut App) -> AnyElement {
        let neutral = tokens.neutral;
        match &self.kind {
            Kind::Line(state) => {
                project(state, tokens, neutral.foreground_muted, cx);
                form_shell(self.focused.get(), tokens, narrow)
                    .child(div().flex_1().child(Input::new(state)))
                    .into_any_element()
            }
            Kind::Lines(state, MultilineStyle::Form) => {
                project(state, tokens, neutral.foreground_muted, cx);
                form_shell(self.focused.get(), tokens, narrow)
                    .flex_col()
                    .items_stretch()
                    .px(px(FORM_INSET_X))
                    .py(px(FORM_INSET_Y))
                    .min_h(px(self.form_minimum.height(narrow)))
                    .max_h(px(FORM_MAX))
                    // The engine emits Enter and propagates the action.
                    // This form already inserted a line or submitted in
                    // its callback; native fallback must not insert again.
                    .on_action(|_: &gpui_base::input::Enter, _window, cx| {
                        cx.stop_propagation();
                    })
                    .child(Textarea::new(state))
                    .into_any_element()
            }
            Kind::Lines(state, MultilineStyle::Composer) => {
                project(state, tokens, neutral.surface4, cx);
                div()
                    .w_full()
                    .min_h(px(COMPOSER_MIN))
                    .type_(Type::Body)
                    .text_size(px(fonts::ui(15.0)))
                    .line_height(px(fonts::ui(21.0)))
                    .text_color(neutral.foreground)
                    .child(Textarea::new(state))
                    .into_any_element()
            }
        }
    }
}

impl TextField {
    /// A line with no chrome of its own, for a row that is its chrome (a
    /// list's search row): 14 on 20, `foregroundMuted` while empty.
    pub fn bare(&self, tokens: &Tokens, cx: &mut App) -> AnyElement {
        let neutral = tokens.neutral;
        let text = div()
            .w_full()
            .type_(Type::Body)
            .line_height(px(fonts::ui(20.0)))
            .text_color(neutral.foreground);
        match &self.kind {
            Kind::Line(state) => {
                project(state, tokens, neutral.foreground_muted, cx);
                text.child(Input::new(state)).into_any_element()
            }
            Kind::Lines(state, _) => {
                project(state, tokens, neutral.foreground_muted, cx);
                text.child(Textarea::new(state)).into_any_element()
            }
        }
    }
}

/// Paseo's composer card (`inputWrapper`): `surface1`, a hairline of
/// `borderAccent`, 16 round; 16 across, 16 above and 12 below (12, 8 and 8
/// on a phone); its text, then its row of controls, 12 apart. It does not
/// change when its text holds the keys.
#[must_use]
pub fn composer_card(tokens: &Tokens, narrow: bool) -> gpui::Div {
    let neutral = tokens.neutral;
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .bg(neutral.surface1)
        .border_1()
        .border_color(neutral.border_accent)
        .rounded(px(16.0))
        .px(px(if narrow { 12.0 } else { 16.0 }))
        .pt(px(if narrow { 8.0 } else { 16.0 }))
        .pb(px(if narrow { 8.0 } else { 12.0 }))
}
