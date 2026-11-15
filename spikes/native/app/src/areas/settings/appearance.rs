//! Appearance, as Paseo's (`screens/settings/appearance/appearance-section.tsx`,
//! `appearance/theme-picker.tsx`): the "Theme" section's tiles, each a
//! small picture of the app in its colours; "Fonts" (the interface's and
//! code's sizes); "Layout" (the content's width). How much of an agent's
//! work the chat shows is Chat's (`chat.rs`), as in Paseo.
//!
//! [Decision Log]
//! - Purpose: Paseo's Appearance, section for section, label for label.
//! - Reference (Paseo): the tiles in a card 12 in, a quarter of its width
//!   each (a half on a phone), 12 apart; a tile 4 in, radius 8, a 2 edge
//!   in the accent when chosen, `surface2` under the pointer; its picture
//!   16:9 in a 1 edge, radius 6: a 36 wide sidebar of three muted lines,
//!   then a title line, two muted lines and the accent's bubble at the
//!   bottom right. System's picture is light on the left and dark on the
//!   right.
//! - Composition: the kit owns the one miniature/tile renderer; this area
//!   supplies preference identities, labels and actions. Remaining font
//!   draft controls are added through the existing field engine.

use theme_tile::{Columns, Preview, State};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Scheme, Tokens, settings, theme_tile};
use vessel_app_view::prefs::{PreferenceChange, Theme};

use super::applied;
use super::choices::List;
use super::general::select_for;
use crate::shell::Shell;

/// The Appearance page.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &gpui::Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.settings();
    let mut grid = theme_tile::grid();
    let columns = Columns::at(window.viewport_size().width);
    for theme in Theme::ALL {
        grid = grid.child(tile(theme, shell, columns, tokens, cx));
    }
    let theme = settings::card(tokens).p(px(12.0)).child(grid);
    let fonts = settings::card(tokens)
        .when(applied(List::Interface), |card| {
            card.child(settings::row_at(
                words.interface_size,
                Some(words.interface_size_note.into()),
                None,
                Some(select_for(List::Interface, shell, compact, tokens, cx)),
                true,
                compact,
                tokens,
            ))
        })
        .when(applied(List::Code), |card| {
            card.child(settings::row_at(
                words.code_size,
                Some(words.code_size_note.into()),
                None,
                Some(select_for(List::Code, shell, compact, tokens, cx)),
                !applied(List::Interface),
                compact,
                tokens,
            ))
        });
    let layout = settings::card(tokens).child(settings::row_at(
        words.content_width,
        Some(words.content_width_note.into()),
        None,
        Some(select_for(List::Width, shell, compact, tokens, cx)),
        true,
        compact,
        tokens,
    ));
    // Each row once what it changes is drawn from it (`super::applied`).
    div()
        .flex()
        .flex_col()
        .child(settings::section(words.theme, theme, tokens))
        .child(super::motion::section(shell, tokens, compact, cx))
        .when(applied(List::Interface) || applied(List::Code), |page| {
            page.child(settings::section(words.fonts, fonts, tokens))
        })
        .when(applied(List::Width), |page| {
            page.child(settings::section(words.layout, layout, tokens))
        })
        .into_any_element()
}

/// Bind a typed preference to the shared token-only tile.
fn tile(
    theme: Theme,
    shell: &Shell,
    columns: Columns,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let state = if shell.prefs.settings.appearance.theme == theme {
        State::Chosen
    } else {
        State::Idle
    };
    let light = Tokens::of(Scheme::Light);
    let dark = Tokens::of(Scheme::Dark);
    let named = super::theme::fixed(theme).map(Tokens::palette);
    let preview = match named.as_ref() {
        Some(tokens) => Preview::Single(tokens),
        None => Preview::Split {
            light: &light,
            dark: &dark,
        },
    };
    let tile = theme_tile::theme_tile(
        gpui::ElementId::Name(format!("settings-theme-{theme:?}").into()),
        shell.t.lang.settings().theme(theme),
        state,
        preview,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        shell.prefer(&PreferenceChange::Theme(theme), window, cx);
    }));
    theme_tile::cell(tile, columns)
}
