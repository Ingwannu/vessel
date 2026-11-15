//! One menu row body, shared by the legacy entries and additive hierarchy.
//! Extracted without changing the reviewed child geometry or paint values.
use super::MenuItem;
use super::draw::{LEADING_SLOT, ROW_LINE, SUCCESS_GLYPH};
use super::item::Leading;
use crate::{fonts::Type, popup, theme::spacing, widgets::TypeExt as _};
use gpui::prelude::*;
use gpui::{AnyElement, ElementId, div, px};

fn leading(index: usize, item: &MenuItem, ink: gpui::Hsla) -> Option<AnyElement> {
    let content = match item.leading() {
        Leading::None => return None,
        Leading::Glyph(glyph) => super::draw::ordinary_glyph(item, glyph, ink).into_any_element(),
        Leading::SuccessCheck => crate::picker::glyph(
            crate::assets::Glyph::CircleCheck,
            f32::from(SUCCESS_GLYPH),
            ink,
        )
        .into_any_element(),
        Leading::Spinner => crate::loading::activity_indicator(
            ElementId::NamedInteger(
                "menu-pending".into(),
                u64::try_from(index).unwrap_or(u64::MAX),
            ),
            crate::loading::Size::Numeric(crate::loading::Points::SIXTEEN),
            ink,
        ),
    };
    Some(
        div()
            .w(LEADING_SLOT)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(content)
            .into_any_element(),
    )
}

pub(super) fn body(
    index: usize,
    item: &MenuItem,
    paint: super::item::Paint,
    narrow: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(ElementId::NamedInteger(
            "menu-row".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ))
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .min_h(px(popup::row_height(narrow)))
        .mx(px(spacing::S1))
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .rounded(px(popup::ROW_RADIUS))
        .type_(Type::Body)
        .line_height(px(crate::fonts::ui(ROW_LINE)))
        .text_color(paint.ink)
        .opacity(paint.opacity)
        .border_1()
        .border_color(gpui::transparent_black())
        .children(leading(index, item, paint.glyph))
        .child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .truncate()
                .child(crate::picker::one_line(item.words())),
        )
}
