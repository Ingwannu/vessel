//! One menu row body, shared by the legacy entries and additive hierarchy.
//! Extracted without changing the reviewed child geometry or paint values.
use super::MenuItem;
use super::draw::{LEADING_SLOT, ROW_LINE, SUCCESS_GLYPH};
use super::hierarchy::{Check, RowProfile};
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

/// Additive slots enter the same frame; a default value is byte-for-behavior
/// compatible with the legacy row body. Selection adds only a check, not fill.
#[derive(Default)]
pub(super) struct Content {
    pub profile: RowProfile,
    pub check: Check,
    pub leading: Option<AnyElement>,
    pub trailing: Option<AnyElement>,
}

// ui/menu/menu-geometry.ts:2; legacy compact44 remains its default profile.
const COMPACT_ROW: f32 = 40.0;
// menu-item.tsx checkSlot and selected trailing mark both use sixteen.
const CHECK_GLYPH: f32 = 16.0;
fn height(profile: RowProfile, narrow: bool) -> f32 {
    match profile {
        RowProfile::Legacy => popup::row_height(narrow),
        RowProfile::Reference if narrow => COMPACT_ROW,
        RowProfile::Reference => popup::row_height(false),
    }
}

pub(super) fn body(
    index: usize,
    item: &MenuItem,
    paint: super::item::Paint,
    narrow: bool,
    surfaces: &popup::Surfaces,
) -> gpui::Stateful<gpui::Div> {
    enhanced(
        ElementId::NamedInteger("menu-row".into(), u64::try_from(index).unwrap_or(u64::MAX)),
        index,
        item,
        paint,
        narrow,
        surfaces,
        Content::default(),
    )
}

pub(super) fn enhanced(
    id: ElementId,
    index: usize,
    item: &MenuItem,
    paint: super::item::Paint,
    narrow: bool,
    surfaces: &popup::Surfaces,
    content: Content,
) -> gpui::Stateful<gpui::Div> {
    frame(index, item, paint, narrow, surfaces, content).id(id)
}

pub(super) fn frame(
    index: usize,
    item: &MenuItem,
    paint: super::item::Paint,
    narrow: bool,
    surfaces: &popup::Surfaces,
    content: Content,
) -> gpui::Div {
    let leading_content = if matches!(item.status, super::ItemStatus::Idle) {
        content
            .leading
            .map(slot)
            .or_else(|| leading(index, item, paint.glyph))
    } else {
        leading(index, item, paint.glyph)
    };
    let check_leading = match content.check {
        Check::Leading(selected) => Some(
            div()
                .w(LEADING_SLOT)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .children(selected.then(|| {
                    crate::picker::glyph(
                        crate::assets::Glyph::Check,
                        CHECK_GLYPH,
                        surfaces.foreground,
                    )
                })),
        ),
        Check::None | Check::Trailing(_) => None,
    };
    let trailing = content.trailing.or_else(|| match content.check {
        Check::Trailing(true) => Some(
            crate::picker::glyph(
                crate::assets::Glyph::Check,
                CHECK_GLYPH,
                surfaces.foreground_muted,
            )
            .into_any_element(),
        ),
        Check::None | Check::Leading(_) | Check::Trailing(false) => None,
    });
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .min_h(px(height(content.profile, narrow)))
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
        .children(check_leading)
        .children(leading_content)
        .child(text(item, surfaces))
        .children(trailing.map(|trailing| {
            div()
                .ml_auto()
                .flex()
                .items_center()
                .justify_center()
                .child(trailing)
        }))
}
fn slot(content: AnyElement) -> AnyElement {
    div()
        .w(LEADING_SLOT)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(content)
        .into_any_element()
}

// menu-item.tsx:355-363,510-514. Native Meta supplies the shared12/16
// metrics where the reference leaves the small-text line height implicit.
const DESCRIPTION_LINES: usize = 2;
fn text(item: &MenuItem, surfaces: &popup::Surfaces) -> gpui::Div {
    let label = div()
        .flex_shrink(1.0)
        .min_w_0()
        .truncate()
        .child(crate::picker::one_line(item.words()));
    match item.visible_description() {
        None => label,
        Some(description) => div()
            .flex_shrink(1.0)
            .min_w_0()
            .flex()
            .flex_col()
            .child(label)
            .child(
                div()
                    .mt(px(spacing::S0_5))
                    .type_(Type::Meta)
                    .text_color(surfaces.foreground_muted)
                    .whitespace_normal()
                    .line_clamp(DESCRIPTION_LINES)
                    .text_ellipsis()
                    .child(description.clone()),
            ),
    }
}
