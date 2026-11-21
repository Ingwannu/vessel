//! A menu row's leading slot (a glyph, an agent's artwork, the pending
//! spinner or the success check) and a group's label row, split from
//! `draw` at its file budget.

use super::MenuItem;
use super::draw::{LEADING_SLOT, SUCCESS_GLYPH};
use super::item::Leading;
use crate::fonts::Type;
use crate::popup::Surfaces;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{AnyElement, ElementId, div, px};

/// A group's label (Paseo's `MenuLabel`: 12 muted, 12 in, 8 above and 4
/// below), in the menu's order; nothing to press.
pub(super) fn heading(index: usize, item: &MenuItem, surfaces: &Surfaces) -> AnyElement {
    div()
        .debug_selector(move || format!("menu-heading-{index}"))
        .px(px(spacing::S3))
        .pt(px(spacing::S2))
        .pb(px(spacing::S1))
        .type_(Type::Meta)
        .text_color(surfaces.foreground_muted)
        .child(crate::picker::one_line(item.words()))
        .into_any_element()
}

pub(super) fn leading(
    index: usize,
    item: &MenuItem,
    ink: gpui::Hsla,
    tokens: &Tokens,
) -> Option<AnyElement> {
    let content = match item.leading() {
        Leading::None => return None,
        Leading::Glyph(glyph) => ordinary_glyph(item, glyph, ink).into_any_element(),
        Leading::Mark(mark) => crate::agent_mark::agent_mark_muted(
            Some(mark),
            item.leading_size.mark(),
            crate::agent_mark::Space::Compact,
            tokens,
        )
        .into_any_element(),
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

// A caller's ordinary glyph size must not scale feedback or its slot.
pub(super) fn ordinary_glyph(
    item: &MenuItem,
    glyph: crate::assets::Glyph,
    ink: gpui::Hsla,
) -> gpui::Svg {
    crate::picker::glyph(glyph, item.leading_size.points(), ink)
}
