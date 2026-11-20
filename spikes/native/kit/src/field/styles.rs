//! Project the kit's colors and type onto the existing editing engines and
//! component theme. No input state, handlers or layout is created here.

use gpui::{App, Entity, px};
use gpui_base::input::InputEditorStyle;

use crate::fonts;
use crate::theme::{Scheme, Tokens};

/// The field's colours, projected onto its engine: the foreground's text,
/// `placeholder` for the words shown while empty, the foreground's caret,
/// a selection of the foreground at a fifth.
pub(super) fn project<M: gpui_base::input::InputModeKind>(
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

/// The component layer's colours and type from the kit's tokens, for what
/// of it is still drawn (Markdown's text).
///
/// [Decision Log]
/// - Purpose: a reply or a previewed file in Markdown draws in the scheme
///   the app shows (TRACKER 825: dark on dark after a switch to a dark
///   palette).
/// - Constraint: the component layer builds its Base theme and the text
///   views' default style (`TextViewDefaults`, which `chat_markdown` starts
///   from) only in `Theme::sync_base`, run by its `init` and its own theme
///   changes. An edit through `Theme::global_mut` reaches neither; its doc
///   says to call `sync_base` after one.
/// - Choice: set the component theme's mode with the scheme and call
///   `sync_base` after the edits, so every palette the app applies (at
///   start, from Settings, or when the system's appearance turns) reaches
///   Markdown, whatever appearance the component layer started under.
/// - Cost: one rebuild of the Base theme per palette change (rare).
pub(crate) fn apply_tokens(cx: &mut App, tokens: &Tokens) {
    let theme = gpui_component::Theme::global_mut(cx);
    theme.mode = match tokens.scheme {
        Scheme::Light => gpui_component::theme::ThemeMode::Light,
        Scheme::Dark => gpui_component::theme::ThemeMode::Dark,
    };
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
    gpui_component::Theme::sync_base(cx);
}
