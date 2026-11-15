//! The select: a setting's value with a caret, opening the list of its
//! values under it (Paseo's `SettingsSelect`: `components/ui/
//! dropdown-trigger.tsx` over its dropdown menu, `side="bottom"`,
//! `align="end"`, 220 wide, a check on the value in effect).
//!
//! [Decision Log]
//! - Purpose: a setting with a few named values (the send shortcut, the
//!   default send, a size, the conversation's width) is chosen from a
//!   list, never from words with a "▾" (the owner's rule after 0.0.0.0.3).
//! - Reference: Paseo's trigger is the outline button's chrome: a
//!   hairline of `borderAccent` on clear, fully round, 32 tall (44 on
//!   touch), 16 across (24 on touch), the value in 14 `foreground` and a 14
//!   muted caret 8 after it, 85% while pressed and half-faded when
//!   disabled; its list opens under it, its right edge on the trigger's.
//! - Choice: the list is the kit's picker list (`picker.rs`'s
//!   [`crate::combobox`]: checked rows, a sheet on a phone, the room
//!   under the trigger as its height), not a second list; this file draws
//!   the trigger and says where the list opens. A screen keeps one
//!   [`crate::ComboboxState`] for all its selects and which one is open,
//!   as the composer's controls share one.
//! - Alternatives: track C's menu (`menu.rs`): Paseo's dropdown, but it has
//!   no checked row, and a select without its check reads as a menu of
//!   actions.

// [Decision Log: HostFilter]
// - Purpose: the schedules host filter is not SettingsSelect's rounded outline.
// - Constraint: existing entry/defaults and one shared renderer must remain.
// - Alternatives: an area-local pill, or overriding a select's padding/paint.
// - Choice: named typed presentation and content; a ready host entry delegates.
// - Why: the same label/anchor/caret retains source-specific geometry and paint.
// - Cost: only the caller owns host identities, status meaning and picker state.
//   HostFilter's source has natural text height +6 vertical padding per side,
//   without a separate touch-size override or an invented open-state wash.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, Bounds, Div, ElementId, Pixels, SharedString, Stateful, div, point, px};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::{ComboHeader, Side, anchor_probe, glyph};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

mod form;
mod style;
use style::{Geometry, PRESSED_OPACITY, Paint, Radius, geometry, opacity, paint};
mod readonly;
pub use form::{
    Activity as FormActivity, Content as FormContent, Value as FormValue, trigger as form_trigger,
};
pub use readonly::readonly_value;

/// Paseo's select list is 220 wide.
pub const WIDTH: f32 = 220.0;

/// Where a trigger was last drawn, in the window.
pub type Anchor = Rc<Cell<Option<Bounds<Pixels>>>>;

/// The reference surface, independent of its current selected value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// Existing `SettingsSelect`: outline, round, 32/44.
    Settings,
    /// `HostFilter`: surface1, radius6, natural body height plus 6+6 padding.
    HostFilter,
    /// `SelectField` MD rectangular form control, independent of loading/value.
    Form,
}
/// Whether an existing select's popover is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Closed.
    Closed,
    /// Open; Settings retains its existing pressed-opacity presentation.
    Open,
}
/// Named options keep the additive API's call sites free of boolean flags.
pub struct TriggerContent {
    /// Whole cached translated value.
    pub label: SharedString,
    /// Caller-owned shared glyph/dot; `HostFilter` keeps its 14-point slot.
    pub leading: Option<AnyElement>,
    /// One of the two reference presentations.
    pub presentation: Presentation,
    /// Existing Settings input density; `HostFilter` does not change by density.
    pub sizing: crate::ControlSizing,
    /// Input availability. Callers attach handlers only while enabled.
    pub availability: crate::ButtonState,
    /// Popover visibility, distinct from pointer hover/press.
    pub visibility: Visibility,
}
impl std::fmt::Debug for TriggerContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerContent")
            .field("label", &self.label)
            .field("presentation", &self.presentation)
            .finish_non_exhaustive()
    }
}

/// Existing select signature/defaults; all drawing delegates to one renderer.
#[must_use]
pub fn trigger(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    anchor: &Anchor,
    open: bool,
    enabled: bool,
    touch: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    trigger_with(
        id,
        TriggerContent {
            label: label.into(),
            leading: None,
            presentation: Presentation::Settings,
            sizing: crate::ControlSizing::from_compact(touch),
            availability: if enabled {
                crate::ButtonState::Enabled
            } else {
                crate::ButtonState::Disabled
            },
            visibility: if open {
                Visibility::Open
            } else {
                Visibility::Closed
            },
        },
        anchor,
        tokens,
    )
}
/// `HostFilter`'s ready trigger; leading is the shared Server glyph or status dot.
/// Its source has the same geometry on pointer/touch and no open-state wash.
#[must_use]
pub fn host_filter(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    leading: AnyElement,
    anchor: &Anchor,
    tokens: &Tokens,
) -> Stateful<Div> {
    trigger_with(
        id,
        TriggerContent {
            label: label.into(),
            leading: Some(leading),
            presentation: Presentation::HostFilter,
            sizing: crate::ControlSizing::Pointer,
            availability: crate::ButtonState::Enabled,
            visibility: Visibility::Closed,
        },
        anchor,
        tokens,
    )
}
/// One shared renderer for legacy Settings and the named `HostFilter` surface.
#[must_use]
pub fn trigger_with(
    id: impl Into<ElementId>,
    content: TriggerContent,
    anchor: &Anchor,
    tokens: &Tokens,
) -> Stateful<Div> {
    render(id.into(), content, None, anchor, tokens)
}
fn render(
    id: ElementId,
    content: TriggerContent,
    form: Option<form::Look>,
    anchor: &Anchor,
    tokens: &Tokens,
) -> Stateful<Div> {
    let g = geometry(content.presentation, content.sizing);
    let paint = paint(content.presentation, tokens);
    let mut trigger = interactions(frame(g, paint).id(id), &content, paint, tokens);
    let field = content.presentation == Presentation::Form;
    if field {
        trigger = trigger.w_full().min_w_0();
    }
    let leading_size = if field {
        FORM_LEADING
    } else {
        crate::theme::icon::SM
    };
    let mut label = label_frame();
    if field {
        label = label.flex_1().text_color(form_foreground(form, tokens));
    }
    trigger
        .child(anchor_probe(anchor))
        .children(content.leading.map(|leading| {
            div()
                .size(px(leading_size))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(leading)
        }))
        .child(label.child(content.label))
        .children(
            form.filter(|look| look.activity == FormActivity::Loading)
                .map(|_| {
                    div()
                        .w(px(crate::theme::icon::MD))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(crate::loading::activity_indicator(
                            "select-field-loading",
                            crate::loading::Size::Numeric(crate::loading::Points::FOURTEEN),
                            tokens.neutral.foreground_muted,
                        ))
                }),
        )
        .child(glyph(
            Glyph::ChevronDown,
            if field {
                crate::theme::icon::MD
            } else {
                crate::theme::icon::SM
            },
            tokens.neutral.foreground_muted,
        ))
}
const FORM_LEADING: f32 = 18.0;
// control-geometry fieldVerticalPadding: (height - line20 - border2)/2.
fn form_foreground(form: Option<form::Look>, tokens: &Tokens) -> gpui::Hsla {
    if form.is_some_and(|look| look.placeholder) {
        tokens.neutral.foreground_muted
    } else {
        tokens.neutral.foreground
    }
}

fn label_frame() -> Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
}
fn frame(g: Geometry, paint: Paint) -> Div {
    let mut trigger = div()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .max_w_full()
        .gap(px(g.gap))
        .px(px(g.horizontal))
        .border_1()
        .border_color(paint.border)
        .bg(paint.background)
        .type_(g.type_)
        .text_color(paint.foreground);
    if let Some(line) = g.line_height {
        trigger = trigger.line_height(px(crate::fonts::ui(line)));
    }
    if let Some(height) = g.minimum {
        trigger = trigger.min_h(px(height));
    }
    if let Some(vertical) = g.vertical {
        trigger = trigger.py(px(vertical));
    }
    trigger = match g.radius {
        Radius::Full => trigger.rounded_full(),
        Radius::Points(value) => trigger.rounded(px(value)),
    };
    trigger
}
fn interactions(
    mut trigger: Stateful<Div>,
    content: &TriggerContent,
    paint: Paint,
    tokens: &Tokens,
) -> Stateful<Div> {
    let enabled = content.availability == crate::ButtonState::Enabled;
    if enabled {
        trigger = trigger.cursor_pointer();
        trigger = match content.presentation {
            Presentation::Settings => trigger.active(|style| style.opacity(PRESSED_OPACITY)),
            Presentation::Form => {
                let accent = tokens.neutral.border_accent;
                trigger
                    .hover(move |style| style.border_color(accent))
                    .active(move |style| style.border_color(accent))
            }
            Presentation::HostFilter => {
                let mut host = trigger;
                if let Some(fill) = paint.hover {
                    host = host.hover(move |style| style.bg(fill));
                }
                if let Some(fill) = paint.pressed {
                    host = host.active(move |style| style.bg(fill));
                }
                host
            }
        };
    }
    if content.presentation == Presentation::Form
        && enabled
        && content.visibility == Visibility::Open
    {
        trigger = trigger.border_color(tokens.neutral.border_accent);
    }
    if let Some(opacity) = opacity(
        content.presentation,
        content.availability,
        content.visibility,
    ) {
        trigger = trigger.opacity(opacity);
    }
    trigger
}
/// The bounds to open a list `width` wide from, for a trigger drawn at
/// `bounds`: Paseo's `align="end"`, the list's right edge on the
/// trigger's (the picker list opens at its anchor's left edge; it keeps
/// itself inside the window).
#[must_use]
pub fn end_aligned(bounds: Bounds<Pixels>, width: f32) -> Bounds<Pixels> {
    let left = width.mul_add(-1.0, f32::from(bounds.right())).max(0.0);
    Bounds {
        origin: point(px(left), bounds.origin.y),
        size: bounds.size,
    }
}

/// A select's list header: no title, no search, under its trigger.
#[must_use]
pub fn header() -> ComboHeader {
    ComboHeader {
        title: None,
        back: false,
        search: false,
        empty: SharedString::default(),
        note: None,
        side: Side::Below,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ControlSizing, theme::Scheme};
    #[test]
    fn public_entries_apply_legacy_opacity_and_host_padding_to_the_actual_frame() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = Tokens::of(scheme);
            let anchor = Anchor::default();
            for touch in [false, true] {
                for open in [false, true] {
                    let mut legacy =
                        trigger("legacy", "Value", &anchor, open, false, touch, &tokens);
                    assert_eq!(legacy.style().opacity, Some(0.5));
                    assert_eq!(
                        legacy.style().min_size.height,
                        Some(px(if touch { 44.0 } else { 32.0 }).into())
                    );
                    assert_eq!(
                        legacy.style().padding.left,
                        Some(px(if touch { 24.0 } else { 16.0 }).into())
                    );
                }
            }
            let mut host = host_filter(
                "host",
                "All machines",
                div().into_any_element(),
                &anchor,
                &tokens,
            );
            assert_eq!(host.style().padding.left, Some(px(12.0).into()));
            assert_eq!(host.style().padding.top, Some(px(6.0).into()));
            assert_eq!(host.style().opacity, None);
            assert_eq!(
                host.style().text.font_weight,
                Some(gpui::FontWeight::MEDIUM)
            );
            assert_eq!(host.style().text.color, Some(tokens.neutral.foreground));
        }
    }
    #[test]
    fn disabled_settings_stay_half_faded_even_while_their_list_is_open() {
        for visibility in [Visibility::Closed, Visibility::Open] {
            assert_eq!(
                opacity(
                    Presentation::Settings,
                    crate::ButtonState::Disabled,
                    visibility
                ),
                Some(0.5)
            );
            for availability in [crate::ButtonState::Enabled, crate::ButtonState::Disabled] {
                assert_eq!(
                    opacity(Presentation::HostFilter, availability, visibility),
                    None
                );
            }
        }
        assert_eq!(
            opacity(
                Presentation::Settings,
                crate::ButtonState::Enabled,
                Visibility::Open
            ),
            Some(0.85)
        );
        assert_eq!(
            opacity(
                Presentation::Settings,
                crate::ButtonState::Enabled,
                Visibility::Closed
            ),
            None
        );
    }
    #[test]
    fn legacy_settings_and_named_host_filter_keep_exact_geometry() {
        for sizing in [ControlSizing::Pointer, ControlSizing::Touch] {
            assert_eq!(
                geometry(Presentation::HostFilter, sizing),
                Geometry {
                    minimum: None,
                    horizontal: 12.0,
                    vertical: Some(6.0),
                    gap: 6.0,
                    radius: Radius::Points(6.0),
                    type_: Type::Label,
                    line_height: None,
                }
            );
            let (height, horizontal) = match sizing {
                ControlSizing::Pointer => (32.0, 16.0),
                ControlSizing::Touch => (44.0, 24.0),
            };
            assert_eq!(
                geometry(Presentation::Settings, sizing),
                Geometry {
                    minimum: Some(height),
                    horizontal,
                    vertical: None,
                    gap: 8.0,
                    radius: Radius::Full,
                    type_: Type::Body,
                    line_height: None,
                }
            );
        }
    }
    #[test]
    fn both_themes_keep_literal_reference_colors_and_legacy_settings_paint() {
        for (scheme, fill, border, hover, pressed, foreground, accent) in [
            (
                Scheme::Light,
                0x00fa_fafa,
                0x00e4_e4e7,
                0x00f4_f4f5,
                0x00e4_e4e7,
                0x001a_1a1e,
                0x00ec_ecf1,
            ),
            (
                Scheme::Dark,
                0x001e_2120,
                0x0025_2b2a,
                0x0027_2a29,
                0x0043_4645,
                0x00fa_fafa,
                0x002f_3534,
            ),
        ] {
            let tokens = Tokens::of(scheme);
            assert_eq!(
                paint(Presentation::Settings, &tokens),
                Paint {
                    background: gpui::transparent_black(),
                    border: gpui::rgb(accent).into(),
                    hover: None,
                    pressed: None,
                    foreground: gpui::rgb(foreground).into()
                }
            );
            assert_eq!(
                paint(Presentation::HostFilter, &tokens),
                Paint {
                    background: gpui::rgb(fill).into(),
                    border: gpui::rgb(border).into(),
                    hover: Some(gpui::rgb(hover).into()),
                    pressed: Some(gpui::rgb(pressed).into()),
                    foreground: gpui::rgb(foreground).into()
                }
            );
        }
    }
}

#[cfg(test)]
mod form_tests;
