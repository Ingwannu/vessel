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
//! - Size: engine wiring and colour projection stay together here so
//!   the three presentations cannot acquire separate handlers; layout
//!   and marked-text regressions live in the separate `form_tests` module.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Entity, Focusable as _, Window, div, px};
use gpui_base::input::{Input, InputEditorStyle, InputEvent, InputState, Textarea, TextareaState};

use crate::fonts::{self, Type};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

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

/// Which press sends a composer's text (Settings › General › Send
/// shortcut, the page's `compose-keys.ts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SendOn {
    /// Enter (and Mod+Enter) sends; Shift+Enter starts a line.
    #[default]
    Enter,
    /// Mod+Enter sends; Enter and Shift+Enter start a line.
    ModEnter,
}

impl SendOn {
    /// Whether an Enter that is not Shift+Enter sends; `with_mod` when
    /// Command (Control elsewhere) was held.
    #[must_use]
    pub const fn sends(self, with_mod: bool) -> bool {
        match self {
            Self::Enter => true,
            Self::ModEnter => with_mod,
        }
    }
}

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
}

impl TextField {
    /// A form field showing `placeholder` while empty. Enter in it calls
    /// `on_submit` on its owner with the text it holds.
    pub fn new<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::line(placeholder, on_submit, None, window, cx)
    }

    /// The same, also calling `on_change` with the text after every edit
    /// (to say what is wrong with it beside the field as it is typed).
    pub fn with_change<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
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
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
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
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self::multiline(MultilineStyle::Form, placeholder, on_submit, window, cx)
    }

    fn multiline<V: 'static>(
        style: MultilineStyle,
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
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
        let shortcut = match style {
            MultilineStyle::Composer => SendOn::Enter,
            MultilineStyle::Form => SendOn::ModEnter,
        };
        let send = Rc::new(Cell::new(shortcut));
        listen(
            &state,
            Handlers {
                on_submit,
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
        Self {
            kind: Kind::Lines(state, style),
            focused,
            send,
        }
    }

    /// Which press sends a composer's text (a form field ignores it).
    pub fn set_send_on(&self, send: SendOn) {
        if matches!(self.kind, Kind::Lines(_, MultilineStyle::Composer)) {
            self.send.set(send);
        }
    }

    fn line<V: 'static>(
        placeholder: &str,
        on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let placeholder = placeholder.to_owned();
        let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let focused = Rc::new(Cell::new(false));
        listen(
            &state,
            Handlers {
                on_submit,
                on_change,
                focused: Rc::clone(&focused),
                send: None,
            },
            window,
            cx,
        );
        Self {
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

    /// Its focus handle: to hear it lose the keys (a title edited in
    /// place saves when the person leaves the field).
    #[must_use]
    pub fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        match &self.kind {
            Kind::Line(state) => state.read(cx).focus_handle(cx),
            Kind::Lines(state, _) => state.read(cx).focus_handle(cx),
        }
    }

    /// Give it the keys.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        let handle = match &self.kind {
            Kind::Line(state) => state.read(cx).focus_handle(cx),
            Kind::Lines(state, _) => state.read(cx).focus_handle(cx),
        };
        handle.focus(window, cx);
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
                    .min_h(px(if narrow { FORM_MIN_TOUCH } else { FORM_MIN }))
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

/// Both form editors use the same surface and border-only focus treatment.
fn form_shell(focused: bool, tokens: &Tokens, narrow: bool) -> gpui::Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .items_center()
        .w_full()
        .min_h(px(if narrow { 44.0 } else { 32.0 }))
        .px(px(12.0))
        .rounded(px(8.0))
        .bg(neutral.surface2)
        .border_1()
        .border_color(if focused {
            neutral.border_accent
        } else {
            gpui::transparent_black()
        })
        .hover(move |style| style.border_color(neutral.border_accent))
        .type_(Type::Body)
        .line_height(px(fonts::ui(20.0)))
        .text_color(neutral.foreground)
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

/// The field's colours, projected onto its engine: the foreground's text,
/// `placeholder` for the words shown while empty, the foreground's caret,
/// a selection of the foreground at a fifth.
fn project<M: gpui_base::input::InputModeKind>(
    state: &Entity<gpui_base::input::InputBaseState<M>>,
    tokens: &Tokens,
    placeholder: gpui::Hsla,
    cx: &mut App,
) {
    let neutral = tokens.neutral;
    let style = InputEditorStyle {
        foreground: neutral.foreground,
        muted_foreground: placeholder,
        caret: neutral.foreground,
        selection: neutral.foreground.opacity(0.2),
        ..InputEditorStyle::default()
    };
    state.update(cx, |state, _cx| state.set_editor_style(style));
}

/// Follow a field's events: Enter, edits, and whether it holds the keys.
/// What a field's events call.
struct Handlers<V: 'static> {
    on_submit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
    on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
    focused: Rc<Cell<bool>>,
    /// A composer's send press; `None` for a form field (Enter submits).
    send: Option<Rc<Cell<SendOn>>>,
}

fn listen<V: 'static, M: gpui_base::input::InputModeKind>(
    state: &Entity<gpui_base::input::InputBaseState<M>>,
    handlers: Handlers<V>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) {
    let Handlers {
        on_submit,
        on_change,
        focused,
        send,
    } = handlers;
    cx.subscribe_in(
        state,
        window,
        move |owner, state, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter {
                shift: false,
                secondary,
            } => {
                if send
                    .as_ref()
                    .is_some_and(|send| !send.get().sends(*secondary))
                {
                    // Enter starts a line here (in place of a selection,
                    // as typing does).
                    state.update(cx, |state, cx| state.replace("\n", window, cx));
                } else {
                    let text = state.read(cx).value().to_string();
                    on_submit(owner, text, window, cx);
                }
            }
            InputEvent::Change => {
                if let Some(on_change) = on_change {
                    let text = state.read(cx).value().to_string();
                    on_change(owner, text, cx);
                }
            }
            InputEvent::Focus => {
                focused.set(true);
                cx.notify();
            }
            InputEvent::Blur => {
                focused.set(false);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {}
        },
    )
    .detach();
}

/// The component layer's colours and type from the kit's tokens, for what
/// of it is still drawn (Markdown's text).
pub(crate) fn apply_tokens(cx: &mut App, tokens: &Tokens) {
    let theme = gpui_component::Theme::global_mut(cx);
    theme.font_family = fonts::SANS.into();
    theme.mono_font_family = fonts::MONO.into();
    theme.font_size = px(fonts::ui(14.0));
    // Code in Markdown at the code size (the page's `--code-px`).
    theme.mono_font_size = px(fonts::code(12.0));
    theme.radius = px(8.0);
    let neutral = tokens.neutral;
    let colors = &mut theme.colors;
    colors.background = tokens.bg_base;
    colors.foreground = neutral.foreground;
    colors.border = neutral.border;
    colors.input = neutral.border_accent;
    colors.ring = gpui::transparent_black();
    colors.caret = neutral.foreground;
    colors.muted_foreground = neutral.foreground_muted;
    colors.selection = neutral.foreground.opacity(0.2);
    colors.popover = neutral.surface0;
    colors.popover_foreground = neutral.foreground;
    colors.accent = neutral.surface2;
    colors.accent_foreground = neutral.foreground;
    // A reply's links: GOAT's `--info-foreground` (its blue), the page's
    // nearest token. Read by `chat_markdown`, which resolves its style
    // from this theme on every layout.
    colors.link = tokens.st_building;
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
mod form_tests;
