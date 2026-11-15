//! Paseo's inline ExternalLink: a label and outward arrow, not a button pill.
//!
//! [Decision Log]
//! - Purpose: CatalogTile's install instructions need their own shared link.
//! - Constraint: ui/external-link.tsx has label12, arrow12, gap4, hitSlop8,
//!   muted ink changing to foreground on hover, no fill or pressed palette.
//! - Alternatives: area-owned pressables, a Ghost button, or OS calls in kit.
//! - Choice: one label/optional-label renderer and expanded target. The caller
//!   owns its typed URL/opener and may use the existing Tooltips wrapper.
//! - Why: drawing is shared; opening a URL is application policy, not layout.
//! - Difference: native Type::Meta supplies its existing12/16 regular metrics;
//!   the reference specifies size12 and leaves line height to the platform.
//! - Cost: accessibility/keyboard link semantics remain the kit-wide gap; this
//!   control does not claim to supply an accessibility tree or an OS opener.
use crate::{
    Tokens, Type, TypeExt as _,
    assets::Glyph,
    theme::{icon, spacing},
};
use gpui::prelude::*;
use gpui::{Div, ElementId, Hsla, MouseButton, SharedString, Stateful, div, px, svg};

// Source values occur once; the hit is the visual plus this inset each side.
const SLOP: f32 = spacing::S2;
const GROUP: &str = "external-link";
const ARROW: f32 = icon::XS;
const GAP: f32 = spacing::S1;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Ink {
    idle: Hsla,
    hover: Hsla,
}
fn ink(tokens: &Tokens) -> Ink {
    Ink {
        idle: tokens.neutral.foreground_muted,
        hover: tokens.neutral.foreground,
    }
}

/// Inline link presentation. `None` draws the arrow alone, as the source does.
/// The caller attaches its opener; kit receives neither a raw nor a typed URL.
#[must_use]
pub fn external_link(
    id: impl Into<ElementId>,
    label: Option<SharedString>,
    tokens: &Tokens,
) -> Stateful<Div> {
    div().id(id).group(GROUP).relative().flex().flex_none()
        .m(px(SLOP.copysign(-1.0))).p(px(SLOP)).cursor_pointer()
        // The reference's Pressable consumes its press. Do not also open the
        // surrounding row when a consumer puts its link inside a press target.
        .on_mouse_up(MouseButton::Left,|_,_,cx| cx.stop_propagation())
        .child(label_row(label,tokens))
}
fn label_row(label: Option<SharedString>, tokens: &Tokens) -> Div {
    let ink = ink(tokens);
    div().flex().items_center().gap(px(GAP))
        .children(label.filter(|text| !text.is_empty()).map(|text| {
            div().type_(Type::Meta).text_color(ink.idle)
                .group_hover(GROUP,move |style| style.text_color(ink.hover)).child(text)
        }))
        // GPUI SVG ink does not inherit from a parent. Put both phases on
        // the actual mask, otherwise only the label changes on hover.
        .child(svg().path(Glyph::ArrowUpRight.path()).size(px(ARROW)).flex_none()
            .text_color(ink.idle).group_hover(GROUP,move |style| style.text_color(ink.hover)))
}
#[cfg(test)]
mod tests;
