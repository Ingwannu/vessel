//! One combobox surface and its placement. Item meaning and selection remain
//! in the parent; this child keeps a typed width extension within the budget.
//!
//! [Decision Log]
//! - Purpose: `HostPicker` grows to its labels, while existing callers fix width.
//! - Constraint: one shared renderer, unchanged legacy API/placement/handlers.
//! - Alternatives: an area-local popup or forcing every label into trigger width.
//! - Choice: fixed or content sizing, minimum the actual measured anchor, maximum
//!   max(400, minimum), fallback200 only when the anchor has not been measured.
//! - Why and cost: matches combobox-frame-style.ts without estimating text; the
//!   screen owns labels and header, GPUI performs intrinsic measurement.
use super::{
    COMPACT, ComboHandlers, ComboHeader, ComboRow, ComboboxState, RenderLease, Side, key_handler,
    note_row, room, rows_list, search_row, title_row,
};
use crate::{
    Tokens, Type, TypeExt as _,
    popup::{self, Surfaces},
};
use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, Bounds, Context, ElementId, MouseButton, Pixels, Window, anchored,
    deferred, div, point, px,
};

/// How far from its trigger a list opens (Paseo's `offset(5)`).
const OFFSET: f32 = 5.0;

/// The same, upward.
const LIFT: f32 = -5.0;

/// How far a list keeps from the window's edges (Paseo's
/// `collisionPadding`).
const EDGE: f32 = 16.0;

/// Explicit legacy width or the reference's measured-anchor content sizing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComboboxWidth {
    /// The old entry's exact fixed logical width.
    Fixed(Pixels),
    /// Grow to labels, within the reference's anchor-derived minimum/maximum.
    Content,
}
const CONTENT_MAX: Pixels = px(400.0);
const UNMEASURED_MIN: Pixels = px(200.0);
#[derive(Debug, Clone, Copy, PartialEq)]
struct Limits {
    minimum: Pixels,
    maximum: Pixels,
}
impl ComboboxWidth {
    fn apply(
        self,
        panel: gpui::Stateful<gpui::Div>,
        anchor: Option<Bounds<Pixels>>,
    ) -> gpui::Stateful<gpui::Div> {
        match self {
            Self::Fixed(width) => panel.w(width),
            Self::Content => {
                let limits = self.limits(anchor);
                panel.min_w(limits.minimum).max_w(limits.maximum)
            }
        }
    }
    fn limits(self, anchor: Option<Bounds<Pixels>>) -> Limits {
        match self {
            Self::Fixed(width) => Limits {
                minimum: width,
                maximum: width,
            },
            Self::Content => {
                let minimum = anchor.map_or(UNMEASURED_MIN, |bounds| bounds.size.width);
                Limits {
                    minimum,
                    maximum: CONTENT_MAX.max(minimum),
                }
            }
        }
    }
}
/// Caller-owned content; grouping keeps the additive entry free of more flags.
#[derive(Debug)]
pub struct ComboboxContent<'a> {
    /// Unique surface identity.
    pub id: ElementId,
    /// Title, search and opening side.
    pub header: &'a ComboHeader,
    /// Already prepared rows in reference order.
    pub rows: &'a [ComboRow],
    /// Default entry uses Fixed; Content is an explicit opt-in.
    pub width: ComboboxWidth,
}
impl<'a> ComboboxContent<'a> {
    /// Construct typed content without changing any row or handler semantics.
    #[must_use]
    pub fn new(
        id: impl Into<ElementId>,
        header: &'a ComboHeader,
        rows: &'a [ComboRow],
        width: ComboboxWidth,
    ) -> Self {
        Self {
            id: id.into(),
            header,
            rows,
            width,
        }
    }
}

/// The list, when open: over everything, above its trigger, `width`
/// wide; a sheet from the bottom below the compact width.
#[expect(
    clippy::too_many_arguments,
    reason = "a list is drawn from its state, its content, its handlers and the window's size"
)]
pub fn combobox<V: 'static>(
    id: impl Into<ElementId>,
    state: &ComboboxState,
    header: &ComboHeader,
    rows: &[ComboRow],
    handlers: &ComboHandlers<V>,
    width: f32,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    combobox_with(
        ComboboxContent::new(id, header, rows, ComboboxWidth::Fixed(px(width))),
        state,
        handlers,
        tokens,
        window,
        cx,
    )
}

/// Typed width through the same surface/placement and input renderer.
pub fn combobox_with<V: 'static>(
    content: ComboboxContent<'_>,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    render(content, state, handlers, None, tokens, window, cx)
}
/// Source model-browser empty states through the same list surface and lease.
pub fn combobox_with_empty<V: 'static>(
    content: ComboboxContent<'_>,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    empty: &super::empty::Empty<V>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    render(content, state, handlers, Some(empty), tokens, window, cx)
}
fn render<V: 'static>(
    content: ComboboxContent<'_>,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    empty: Option<&super::empty::Empty<V>>,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    let ComboboxContent {
        id: list_id,
        header,
        rows,
        width,
    } = content;
    if !state.open {
        return None;
    }
    let compact = window.viewport_size().width < px(COMPACT);
    let surfaces = Surfaces::of(tokens);
    let neutral = tokens.neutral;
    let get = handlers.state;
    let on_key = key_handler(rows, state, handlers, cx);
    let lease = state.render_lease();
    let outside_lease = lease.clone();
    let on_outside = cx.listener(move |view: &mut V, _event, window, cx| {
        if get(view).accepts(&outside_lease) {
            get(view).close_and_restore(window, cx);
            cx.notify();
        } else {
            window.prevent_default();
        }
        cx.stop_propagation();
    });

    let mut panel = div()
        .id(list_id.clone())
        .track_focus(&state.focus)
        .capture_any_mouse_down(cx.listener(move |view: &mut V, _event, window, cx| {
            if !get(view).accepts(&lease) {
                window.prevent_default();
                cx.stop_propagation();
            }
        }))
        .capture_key_down(on_key)
        // The popover takes the pointer and shows the arrow (`popup::layer`).
        .map(popup::layer)
        .flex()
        .flex_col()
        .bg(surfaces.surface0)
        .border_1()
        .border_color(neutral.border)
        .overflow_hidden()
        .type_(Type::Body)
        .text_color(surfaces.foreground);
    let anchor = state.anchor.get();
    let room = room(anchor, header.side, window);
    panel = if compact {
        panel
            .w_full()
            .max_h(gpui::relative(0.9))
            .rounded_t(px(popup::SHEET_RADIUS))
            .pb(px(16.0))
    } else {
        width
            .apply(panel, anchor)
            .max_h(px(room))
            .rounded(px(popup::MENU_RADIUS))
            .shadow(popup::shadow_md(tokens))
            .on_mouse_down_out(on_outside)
    };
    panel = list_header(panel, header, state, handlers, tokens, cx);
    let body = if let Some(empty) = empty.filter(|_| rows.is_empty()) {
        super::empty::render(empty, state, handlers, compact, tokens, cx)
    } else {
        rows_list(header, rows, state, handlers, compact, tokens, cx)
    };
    panel = panel.child(body);

    // Word 39: the list arrives as Paseo's popover does (opacity and a
    // small rise, `popup::appear`); a phone's sheet over the surface
    // length. It leaves at once, as Paseo's desktop popovers do.
    let panel = popup::appear(
        ElementId::NamedChild(std::sync::Arc::new(list_id), "appear".into()),
        panel,
        if compact {
            crate::motion::STANDARD
        } else {
            crate::motion::QUICK
        },
    );
    let placement = Placement {
        compact,
        side: header.side,
        anchor,
        lease: state.render_lease(),
    };
    let element = placed(panel, placement, get, window, cx);
    // Over a dialog too (`kit::dialog` is at 2): a feature's list opens
    // from a row of the features sheet.
    // A row's tooltip draws over the list (the tooltip's own priority).
    let tips = state.tips.render(tokens);
    Some(
        div()
            .child(deferred(element).with_priority(3))
            .children(tips)
            .into_any_element(),
    )
}

/// Shared title/search/note in their unchanged order, separate from placement.
fn list_header<V: 'static>(
    mut panel: gpui::Stateful<gpui::Div>,
    header: &ComboHeader,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> gpui::Stateful<gpui::Div> {
    if let Some(title) = &header.title {
        panel = panel.child(title_row(
            title.clone(),
            header.back,
            state,
            handlers,
            tokens,
            cx,
        ));
    }
    if header.search {
        panel = panel.child(search_row(state, tokens, cx));
    }
    if let Some(note) = &header.note {
        panel = panel.child(note_row(note.clone(), tokens));
    }
    panel
}

/// Where the list is drawn: below the compact width a sheet from the
/// bottom over the dimmed page (a press on the dim closes it); wider, at
/// its trigger on the caller's side.
struct Placement {
    compact: bool,
    side: Side,
    anchor: Option<Bounds<Pixels>>,
    lease: RenderLease,
}
fn placed<V: 'static>(
    panel: impl IntoElement,
    placement: Placement,
    get: fn(&mut V) -> &mut ComboboxState,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let Placement {
        compact,
        side,
        anchor,
        lease,
    } = placement;
    if compact {
        // A sheet: the page dimmed behind, the list from the bottom.
        let viewport = window.viewport_size();
        let on_scrim = cx.listener(move |view: &mut V, _event, window, cx| {
            if get(view).accepts(&lease) {
                get(view).close_and_restore(window, cx);
                cx.notify();
            } else {
                window.prevent_default();
            }
            cx.stop_propagation();
        });
        anchored()
            .position(point(px(0.0), px(0.0)))
            .child(
                div()
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .flex_col()
                    .justify_end()
                    .bg(popup::sheet_scrim())
                    .on_mouse_down(MouseButton::Left, on_scrim)
                    .child(panel),
            )
            .into_any_element()
    } else {
        // Paseo's `offset(5)` from the trigger, on the caller's side.
        let (at, corner, offset) = match (side, anchor) {
            // Not drawn yet (the first frame): near the window's middle.
            (_, None) => (point(px(8.0), px(240.0)), Anchor::BottomLeft, 0.0),
            (Side::Above, Some(bounds)) => (bounds.origin, Anchor::BottomLeft, LIFT),
            (Side::Below, Some(bounds)) => (bounds.bottom_left(), Anchor::TopLeft, OFFSET),
        };
        anchored()
            .position(at)
            .offset(point(px(0.0), px(offset)))
            .anchor(corner)
            .snap_to_window_with_margin(px(EDGE))
            .child(panel)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_panel_styles_keep_fixed_width_and_content_bounds_separate() {
        let mut fixed = ComboboxWidth::Fixed(px(220.0)).apply(div().id("fixed"), None);
        assert_eq!(fixed.style().size.width, Some(px(220.0).into()));
        assert_eq!(fixed.style().min_size.width, None);
        assert_eq!(fixed.style().max_size.width, None);
        let anchor = Bounds {
            origin: point(px(17.0), px(20.0)),
            size: gpui::size(px(280.0), px(32.0)),
        };
        let mut content = ComboboxWidth::Content.apply(div().id("content"), Some(anchor));
        assert_eq!(content.style().size.width, None);
        assert_eq!(content.style().min_size.width, Some(px(280.0).into()));
        assert_eq!(content.style().max_size.width, Some(px(400.0).into()));
    }
    #[test]
    fn content_uses_measured_anchor_and_grows_past_the_default_cap_if_needed() {
        for (anchor, minimum, maximum) in [
            (None, 200.0, 400.0),
            (Some(120.0), 120.0, 400.0),
            (Some(280.0), 280.0, 400.0),
            (Some(520.0), 520.0, 520.0),
        ] {
            let anchor = anchor.map(|width| Bounds {
                origin: point(px(17.0), px(20.0)),
                size: gpui::size(px(width), px(32.0)),
            });
            assert_eq!(
                ComboboxWidth::Content.limits(anchor),
                Limits {
                    minimum: px(minimum),
                    maximum: px(maximum)
                }
            );
            assert_eq!(
                ComboboxWidth::Fixed(px(220.0)).limits(anchor),
                Limits {
                    minimum: px(220.0),
                    maximum: px(220.0)
                }
            );
        }
    }
}
