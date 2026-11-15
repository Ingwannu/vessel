//! Paseo's `SidebarHeaderRow`, shared by builtin and plugin navigation.
//! References: sidebar-header-row.tsx:47–112,124–201; density.ts:25–32.
//!
//! [Decision Log]
//! One renderer owns inset, density, selected/hover/pressed and shortcut
//! behavior. Callers own navigation and the optional trailing content. A
//! null icon keeps the same slot; settings can reuse Header later without
//! copying its separator or window chrome (which stays with the shell).

pub use crate::screen_header::Density;
use crate::{
    Tokens, Type, TypeExt as _,
    assets::Glyph,
    picker,
    theme::{radius, spacing},
};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, ClickEvent, Div, FocusHandle, FontWeight, Hsla, SharedString, Window, div, px,
};

/// Wrapper geometry from the reference, independent of selected state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// A full-height back/header strip with a bottom separator.
    Header,
    /// A tight row in a group that owns its divider.
    Compact,
    /// Full-width content in an already padded footer.
    Inline,
}
/// An active destination uses the same highlighted fill as hover.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Selection {
    /// The destination is elsewhere.
    #[default]
    Inactive,
    /// This row names the active destination.
    Active,
}

/// Caller-owned slots; no routes or application state enter the kit.
#[must_use]
pub struct SidebarHeaderRow {
    id: SharedString,
    label: SharedString,
    variant: Variant,
    selection: Selection,
    leading: Option<Glyph>,
    trailing: Option<AnyElement>,
    shortcut: Option<AnyElement>,
    focus: Option<FocusHandle>,
}
impl SidebarHeaderRow {
    /// Create one row with a stable unique id and whole translated label.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        variant: Variant,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant,
            selection: Selection::Inactive,
            leading: None,
            trailing: None,
            shortcut: None,
            focus: None,
        }
    }
    /// Highlight the active destination, in addition to pointer hover.
    pub fn selected(mut self, selection: Selection) -> Self {
        self.selection = selection;
        self
    }
    /// Supply the shared glyph; without one the icon slot remains.
    pub fn leading(mut self, glyph: Glyph) -> Self {
        self.leading = Some(glyph);
        self
    }
    /// A persistent right slot takes precedence over the hover-only hint.
    pub fn trailing(mut self, element: AnyElement) -> Self {
        self.trailing = Some(element);
        self
    }
    /// A caller-prepared current shortcut, shown only while hovered.
    pub fn shortcut(mut self, element: AnyElement) -> Self {
        self.shortcut = Some(element);
        self
    }
    /// Optional existing focus target; keyboard activation stays with caller.
    pub fn focus(mut self, focus: FocusHandle) -> Self {
        self.focus = Some(focus);
        self
    }

    /// Draw one row and bind its navigation once. Trailing controls may stop
    /// propagation themselves; there is no nested button added around them.
    pub fn element(
        self,
        density: Density,
        tokens: &Tokens,
        press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Div {
        let geometry = geometry(self.variant, density);
        let normal = paint(self.selection, Phase::Idle, tokens);
        let hover = paint(self.selection, Phase::Hovered, tokens);
        let pressed = paint(self.selection, Phase::Pressed, tokens);
        let mut row = div()
            .id(self.id.clone())
            .group(self.id.clone())
            .flex()
            .items_center()
            .min_h(px(geometry.row))
            .rounded(px(radius::R2))
            .cursor_pointer()
            .bg(normal.fill)
            .text_color(normal.ink)
            .hover(move |style| style.bg(hover.fill).text_color(hover.ink))
            .active(move |style| style.bg(pressed.fill))
            .on_click(press);
        if let Some(focus) = self.focus {
            row = row.track_focus(&focus);
        }
        let icon = match self.leading {
            Some(glyph) => picker::glyph(glyph, geometry.icon, normal.ink)
                .group_hover(self.id.clone(), move |style| style.text_color(hover.ink))
                .into_any_element(),
            None => div().size(px(geometry.icon)).flex_none().into_any_element(),
        };
        // Label supplies the kit's 14/20 text metrics; normal weight is the
        // reference's body text, keeping the complete row at 28/36 points.
        row = row.child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(spacing::S2))
                .min_h(px(geometry.row))
                .py(px(spacing::S1))
                .px(px(spacing::S2))
                .type_(Type::Label)
                .font_weight(FontWeight::NORMAL)
                .child(icon)
                .child(
                    div()
                        .min_w_0()
                        .flex_shrink_1()
                        .overflow_hidden()
                        .child(self.label),
                ),
        );
        if let Some(trailing) = self.trailing {
            row = row.child(trailing_frame(trailing));
        } else if let Some(shortcut) = self.shortcut {
            row = row.child(
                trailing_frame(shortcut)
                    .hidden()
                    .group_hover(self.id, |style| style.flex()),
            );
        }
        let mut outer = div()
            .flex()
            .flex_col()
            .flex_none()
            .justify_center()
            .w_full();
        match self.variant {
            Variant::Header => {
                outer = outer
                    .h(px(geometry.header))
                    .px(px(spacing::S2))
                    .border_b_1()
                    .border_color(tokens.neutral.border)
            }
            Variant::Compact => outer = outer.px(px(spacing::S2)),
            Variant::Inline => {}
        }
        outer.child(row)
    }
}
fn trailing_frame(element: AnyElement) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(spacing::S1))
        .pr(px(spacing::S2))
        .child(element)
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    row: f32,
    header: f32,
    icon: f32,
}
const fn geometry(variant: Variant, density: Density) -> Geometry {
    match (density, variant) {
        (Density::Desktop, Variant::Header) => Geometry {
            row: 28.0,
            header: 36.0,
            icon: 16.0,
        },
        (Density::Desktop, Variant::Compact | Variant::Inline) => Geometry {
            row: 28.0,
            header: 36.0,
            icon: 14.0,
        },
        (Density::Compact, Variant::Header) => Geometry {
            row: 36.0,
            header: 56.0,
            icon: 20.0,
        },
        (Density::Compact, Variant::Compact | Variant::Inline) => Geometry {
            row: 36.0,
            header: 56.0,
            icon: 18.0,
        },
    }
}
#[derive(Clone, Copy)]
enum Phase {
    Idle,
    Hovered,
    Pressed,
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct Paint {
    fill: Hsla,
    ink: Hsla,
}
fn paint(selection: Selection, phase: Phase, tokens: &Tokens) -> Paint {
    let highlighted = selection == Selection::Active || matches!(phase, Phase::Hovered);
    let hover = match tokens.scheme {
        crate::Scheme::Light => tokens.neutral.surface3,
        crate::Scheme::Dark => tokens.neutral.surface1,
    };
    Paint {
        fill: if matches!(phase, Phase::Pressed) {
            tokens.neutral.surface2
        } else if highlighted {
            hover
        } else {
            gpui::transparent_black()
        },
        ink: if highlighted {
            tokens.neutral.foreground
        } else {
            tokens.neutral.foreground_muted
        },
    }
}

#[cfg(test)]
mod tests;
