//! The sidebar's navigation rows: its own pages' rows, the slots' rows
//! (Schedules) and the plugins' rows share one drawing.

use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    Div, ElementId, FluentBuilder as _, InteractiveElement as _, ParentElement as _, SharedString,
    Stateful, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};

use super::INSET;

pub(super) fn row(
    id: impl Into<ElementId>,
    label: &'static str,
    active: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    nav_row(id, label, None, active, false, tokens)
}

/// A navigation row of the sidebar, for an entry in its slots (track G2's
/// Schedules, after Paseo's `SidebarHeaderRow` `compact`): `leading`, a
/// glyph before the words in the words' colour, 8 from them (Paseo's
/// `spacing[2]`), at Paseo's `sm` density icon: 14 points, 18 on `touch`
/// (`control-geometry.ts`, `ICON_SIZE` and `TOUCH_ICON_SIZE`). `None`
/// draws the row exactly as the sidebar's own rows were drawn (the flex
/// row is set only with a glyph). The caller adds what a press does.
///
/// Differences from Paseo's row, named, for the sidebar's full drawing
/// (track S's slice) to settle for every row at once: a hovered row here
/// only fills (Paseo also turns the words and the glyph to the
/// foreground); the row's padding, type and radius are the sidebar's own,
/// not Paseo's compact values; no shortcut on hover.
pub(crate) fn nav_row(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    leading: Option<vessel_app_kit::assets::Glyph>,
    active: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let color = if active {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };
    let glyph_size = if touch { 18.0 } else { 14.0 };
    div()
        .id(id)
        .when(leading.is_some(), |row| {
            row.flex().flex_row().items_center().gap(px(spacing::S2))
        })
        .px(px(INSET))
        .py(px(spacing::S1_5))
        .rounded(px(radius::R4))
        .type_(Type::Label)
        .text_color(color)
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .children(leading.map(|glyph| {
            vessel_app_kit::ui::svg()
                .path(glyph.path())
                .size(px(glyph_size))
                .flex_none()
                .text_color(color)
        }))
        .child(label.into())
}
