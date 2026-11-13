//! Appearance, as the page's (`features/settings/appearance.tsx`) in
//! Paseo's layout (`screens/settings/appearance/appearance-section.tsx`,
//! `appearance/theme-picker.tsx`): the theme as tiles, each a small
//! picture of the app in its colours; the interface's and code's sizes;
//! the conversation's width and display.
//!
//! [Decision Log]
//! - Purpose: the page's Appearance, every row of it.
//! - Reference (Paseo): the tiles in a card 12 in, a quarter of its width
//!   each (a half on a phone), 12 apart; a tile 4 in, radius 8, a 2 edge
//!   in the accent when chosen, `surface2` under the pointer; its picture
//!   16:9 in a 1 edge, radius 6: a 36 wide sidebar of three muted lines,
//!   then a title line, two muted lines and the accent's bubble at the
//!   bottom right. System's picture is light on the left and dark on the
//!   right.
//! - Differences: the sizes and the width are selects over the model's
//!   values (Paseo types a number of pixels): the model keeps closed
//!   ranges (`prefs/appearance.rs`), so a choice cannot be out of them.
//!   The sizes are kept, not yet applied: the kit's type steps are
//!   constants today.

use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px, relative,
};
use vessel_app_kit::{Scheme, Tokens, Type, TypeExt as _, settings};
use vessel_app_view::prefs::{PreferenceChange, Theme};

use super::applied;
use super::choices::List;
use super::general::select_for;
use crate::shell::Shell;

/// A picture's width over its height (16:9).
const PICTURE_RATIO: f32 = 1.777_777_8;

/// The Appearance page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.settings();
    let mut grid = div().flex().flex_row().flex_wrap().m(px(-6.0));
    for theme in Theme::ALL {
        grid = grid.child(tile(theme, shell, compact, tokens, cx));
    }
    let theme = settings::card(tokens).p(px(12.0)).child(grid);
    let typography = settings::card(tokens)
        .child(settings::row_at(
            words.interface_font,
            Some(words.interface_font_note.into()),
            None,
            Some(select_for(List::Interface, shell, compact, tokens, cx)),
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.code_font,
            Some(words.code_font_note.into()),
            None,
            Some(select_for(List::Code, shell, compact, tokens, cx)),
            false,
            compact,
            tokens,
        ));
    let conversation = settings::card(tokens)
        .child(settings::row_at(
            words.content_width,
            Some(words.content_width_note.into()),
            None,
            Some(select_for(List::Width, shell, compact, tokens, cx)),
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.conversation_display,
            Some(words.conversation_display_note.into()),
            None,
            Some(select_for(List::Display, shell, compact, tokens, cx)),
            false,
            compact,
            tokens,
        ));
    // The sizes and the conversation's rows only once what they change is
    // drawn from them (`super::applied`): the kit's type steps are fixed
    // and the conversation's column does not read its width yet.
    let sizes = applied(List::Interface) || applied(List::Code);
    let conversation_shown = applied(List::Width) || applied(List::Display);
    div()
        .flex()
        .flex_col()
        .child(settings::section(words.theme, theme, tokens))
        .when(sizes, |page| {
            page.child(settings::section(words.typography, typography, tokens))
        })
        .when(conversation_shown, |page| {
            page.child(settings::section(words.conversation, conversation, tokens))
        })
        .into_any_element()
}

/// `theme`'s tile.
fn tile(
    theme: Theme,
    shell: &Shell,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let chosen = shell.prefs.settings.appearance.theme == theme;
    let picture = match theme {
        Theme::System => system_picture(),
        Theme::Light => picture(&Tokens::of(Scheme::Light)),
        Theme::Dark => picture(&Tokens::of(Scheme::Dark)),
    };
    div()
        .w(relative(if compact { 0.5 } else { 0.25 }))
        .p(px(6.0))
        .child(
            div()
                .id(gpui::ElementId::Name(
                    format!("settings-theme-{theme:?}").into(),
                ))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(4.0))
                .rounded(px(8.0))
                .border_2()
                .border_color(if chosen {
                    neutral.accent
                } else {
                    gpui::transparent_black()
                })
                .when(!chosen, |tile| {
                    tile.hover(move |style| style.bg(neutral.surface2))
                })
                .cursor_pointer()
                .child(
                    div()
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(neutral.border)
                        .overflow_hidden()
                        .child(picture),
                )
                .child(
                    div()
                        .px(px(4.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .type_(Type::Body)
                        .text_color(neutral.foreground)
                        .child(shell.t.lang.settings().theme(theme)),
                )
                .on_click(cx.listener(move |shell, _event, window, cx| {
                    shell.prefer(&PreferenceChange::Theme(theme), window, cx);
                })),
        )
}

/// A line of a picture: `width` of its row, `height` tall, in `color`.
fn line(width: f32, height: f32, color: gpui::Hsla) -> Div {
    div()
        .w(relative(width))
        .h(px(height))
        .rounded(px(2.0))
        .bg(color)
}

/// The app in `tokens`' colours, small.
fn picture(tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let muted = neutral.foreground_muted.opacity(0.6);
    div()
        .w_full()
        .aspect_ratio(PICTURE_RATIO)
        .flex()
        .flex_row()
        .overflow_hidden()
        .bg(neutral.surface0)
        .child(
            div()
                .w(px(36.0))
                .h_full()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .px(px(6.0))
                .py(px(8.0))
                .bg(neutral.surface_sidebar)
                .child(line(1.0, 4.0, muted))
                .child(line(0.6, 4.0, muted))
                .child(line(0.8, 4.0, muted)),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .p(px(8.0))
                .child(line(0.55, 5.0, neutral.foreground))
                .child(line(0.9, 4.0, muted))
                .child(line(0.65, 4.0, muted))
                .child(
                    div()
                        .mt_auto()
                        .self_end()
                        .w(relative(0.45))
                        .h(px(10.0))
                        .rounded(px(5.0))
                        .bg(neutral.accent),
                ),
        )
}

/// System's picture: light, its right half dark.
fn system_picture() -> Div {
    div()
        .relative()
        .w_full()
        .child(picture(&Tokens::of(Scheme::Light)))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(0.5))
                .w(relative(0.5))
                .overflow_hidden()
                .child(
                    div()
                        .w(relative(2.0))
                        .ml(relative(-1.0))
                        .child(picture(&Tokens::of(Scheme::Dark))),
                ),
        )
}
