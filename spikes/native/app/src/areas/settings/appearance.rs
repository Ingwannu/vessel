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

mod fields;
mod preview;
mod rows;
mod syntax;
pub(super) use syntax::{label as syntax_label, palette as syntax_palette};

use gpui::{App, Window};
use vessel_app_kit::code_preview::{PreviewError, PreviewRow, PreviewTypography};
use vessel_app_view::prefs::Appearance;

/// Inputs and prepared local preview; the text engine alone owns raw drafts.
pub(super) struct State {
    fields: fields::Fields,
    saved: Option<Appearance>,
    preview: Option<PreviewTypography>,
    rows: Result<Vec<PreviewRow>, PreviewError>,
}
impl State {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            fields: fields::Fields::new(window, cx),
            saved: None,
            preview: None,
            rows: preview::rows(),
        }
    }
}
/// Hydrate/selectively resync outside paint, retaining unrelated live drafts.
pub(super) fn sync(shell: &mut Shell, window: &mut Window, cx: &mut App) {
    let placeholder = shell.words.appearance().default_family();
    for field in [
        vessel_app_view::appearance::form::Field::InterfaceFamily,
        vessel_app_view::appearance::form::Field::MonoFamily,
    ] {
        shell
            .areas
            .settings
            .appearance
            .fields
            .of(field)
            .set_placeholder(placeholder, window, cx);
    }
    let full = shell
        .t
        .lang
        .settings()
        .width(vessel_app_view::prefs::ContentWidth::Full);
    shell
        .areas
        .settings
        .appearance
        .sync(&shell.prefs.settings.appearance, full, window, cx);
}

use theme_tile::{Columns, Preview, State as TileState};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Scheme, Tokens, settings, theme_tile};
use vessel_app_view::prefs::{PreferenceChange, Theme};

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
    div()
        .flex()
        .flex_col()
        .child(settings::section(words.theme, theme, tokens))
        .child(super::motion::section(shell, tokens, compact, cx))
        .child(rows::fonts(shell, tokens, compact, cx))
        .child(rows::layout(shell, tokens, compact, cx))
        .child(rows::syntax(shell, tokens, compact, cx))
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
        TileState::Chosen
    } else {
        TileState::Idle
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

#[cfg(test)]
mod tests;
