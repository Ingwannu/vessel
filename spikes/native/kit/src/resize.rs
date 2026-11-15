//! A side panel's resize handle, for the sidebar at the left and the code
//! panel at the right (the owner, 2026-11-14, word 29: both resizable by
//! dragging their inner edge). Paseo's handle (`sidebar-resize-handle.tsx`
//! and its layout file): a pointer strip 10 wide straddling the panel's
//! edge, the column-resize pointer over it, and after 100 ms of hovering a
//! line 1 wide in the ink at 25 % down the edge.
//!
//! [Decision Log]
//! - Purpose: one control for both panels; the width rule is the caller's
//!   (`view::panels`), the handle only reports where the pointer is.
//! - Constraint: GPUI's drag API (`on_drag` / `on_drag_move`, the code
//!   panel's first version) leaves the pointer's shape and the elements
//!   under it to whatever the pointer passes over: off the 6-wide strip the
//!   pointer became the text cursor and the conversation's text could be
//!   selected.
//! - Choice: a press on the strip starts the drag and a shield covers the
//!   window until the release (a `popup::layer`): it holds the
//!   column-resize pointer, takes every move and the release, and nothing
//!   under it is hovered, pressed or selected. The panel's bounds, recorded
//!   each frame ([`measure`]), turn the pointer into the width asked for.
//!   A double-click returns to the default (an addition: Paseo has none).
//! - Differences, named: no touch grip (Paseo's touch variant is 24 by 88
//!   inside the panel; the desktop app has a pointer).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, DispatchPhase, ElementId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Window, anchored, canvas, deferred, div, point, px,
};

use crate::popup;
use crate::theme::Tokens;

/// The strip's width, straddling the edge.
const STRIP: f32 = 10.0;
/// The hover line's width and opacity.
const LINE: f32 = 1.0;
const LINE_OPACITY: f32 = 0.25;
/// How long the pointer rests before the line shows.
const HIGHLIGHT_DELAY: Duration = Duration::from_millis(100);

/// Which edge of its panel the handle is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// The panel's right edge: it grows to the right (the sidebar).
    Right,
    /// The panel's left edge: it grows to the left (the code panel).
    Left,
}

/// A handle's state, kept by its screen.
#[derive(Debug, Clone, Default)]
pub struct ResizeState {
    /// The panel's bounds last frame.
    panel: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// While dragging.
    dragging: Rc<Cell<bool>>,
    /// The hover line shows.
    lit: Rc<Cell<bool>>,
    /// The pointer is on the strip (the line waits for it to rest).
    over: Rc<Cell<bool>>,
}

impl ResizeState {
    /// A handle at rest.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a drag is in progress.
    #[must_use]
    pub fn dragging(&self) -> bool {
        self.dragging.get()
    }

    /// The panel's width as drawn last frame (`None` before it is drawn).
    #[must_use]
    pub fn width(&self) -> Option<f32> {
        self.panel.get().map(|bounds| f32::from(bounds.size.width))
    }
}

/// A callback with the width asked for, in points.
pub type OnWidth = Rc<dyn Fn(f32, &mut Window, &mut App)>;
/// A callback.
pub type OnDone = Rc<dyn Fn(&mut Window, &mut App)>;

/// What the handle reports.
#[derive(Clone)]
#[expect(missing_debug_implementations, reason = "callbacks only")]
pub struct ResizeEvents {
    /// The width asked for while dragging.
    pub drag: OnWidth,
    /// The drag ended (keep the width).
    pub done: OnDone,
    /// A double-click: back to the default.
    pub reset: OnDone,
}

/// Records the panel's bounds; put inside the panel (which is `relative`).
#[must_use]
pub fn measure(state: &ResizeState) -> AnyElement {
    let panel = Rc::clone(&state.panel);
    canvas(
        move |bounds, _window, _cx| panel.set(Some(bounds)),
        |_, (), _, _| {},
    )
    .absolute()
    .size_full()
    .into_any_element()
}

/// The width a pointer at `x` asks for, the panel at `bounds` resized at
/// its `edge`.
fn asked(x: Pixels, bounds: Bounds<Pixels>, edge: Edge) -> f32 {
    let x = f32::from(x);
    match edge {
        Edge::Right => f32::from(bounds.left()).mul_add(-1.0, x),
        Edge::Left => x.mul_add(-1.0, f32::from(bounds.right())),
    }
}

/// The handle on the panel's `edge`; put inside the panel (which is
/// `relative`), after its content. Drawn after the panes beside it
/// (deferred, under every popover), so the half of the strip over the
/// neighbour's edge is not covered by it. While dragging it draws the
/// shield too.
#[must_use]
pub fn handle(
    id: impl Into<ElementId>,
    state: &ResizeState,
    edge: Edge,
    tokens: &Tokens,
    events: &ResizeEvents,
    window: &Window,
) -> AnyElement {
    let half = px(STRIP.mul_add(-0.5, 0.0));
    let shown = state.lit.get() || state.dragging.get();
    let line = div()
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(STRIP.mul_add(0.5, 0.0)))
        .w(px(LINE))
        .bg(tokens.neutral.foreground.opacity(LINE_OPACITY));
    let strip = div()
        .id(id.into())
        .absolute()
        .top_0()
        .bottom_0()
        .w(px(STRIP))
        .cursor_col_resize()
        .when(shown, |strip| strip.child(line));
    let strip = match edge {
        Edge::Right => strip.right(half),
        Edge::Left => strip.left(half),
    };
    let (over, lit) = (Rc::clone(&state.over), Rc::clone(&state.lit));
    let (dragging, ended) = (Rc::clone(&state.dragging), Rc::clone(&state.dragging));
    let reset = Rc::clone(&events.reset);
    let done = Rc::clone(&events.done);
    let strip = strip
        .on_hover(move |hovered, window, cx| {
            over.set(*hovered);
            if !*hovered {
                lit.set(false);
                window.refresh();
                return;
            }
            // Paseo lights the line once the pointer rests 100 ms.
            let (over, lit) = (Rc::clone(&over), Rc::clone(&lit));
            window
                .spawn(cx, async move |cx| {
                    cx.background_executor().timer(HIGHLIGHT_DELAY).await;
                    if over.get() {
                        lit.set(true);
                        let _drawn = cx.update(|window, _cx| window.refresh());
                    }
                })
                .detach();
        })
        // The press starts the drag (and keeps it from the text under the
        // strip). The second press of a double-click resets instead: the
        // shield is over the strip by the release, so a click never
        // completes on it.
        .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            if event.click_count == 2 {
                dragging.set(false);
                reset(window, cx);
            } else {
                dragging.set(true);
            }
            window.refresh();
        })
        // Released before the shield was drawn (a quick click).
        .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
            if ended.replace(false) {
                done(window, cx);
                window.refresh();
            }
        });
    // Over the whole panel, so the strip is placed at its edge; no hitbox
    // of its own.
    div()
        .absolute()
        .inset_0()
        .child(deferred(strip))
        .children(shield(state, edge, events, window))
        .into_any_element()
}

/// While dragging: a layer over the whole window that keeps the resize
/// pointer and lets nothing under it be hovered, pressed or selected; it
/// takes every move (each a width asked for) and the release, wherever
/// the pointer is (listened to on the window, not on the layer's
/// hitbox, so a release past the window's edge still ends the drag when
/// the system reports it). `None` at rest.
fn shield(
    state: &ResizeState,
    edge: Edge,
    events: &ResizeEvents,
    window: &Window,
) -> Option<AnyElement> {
    if !state.dragging.get() {
        return None;
    }
    let size = window.viewport_size();
    let panel = Rc::clone(&state.panel);
    let drag = Rc::clone(&events.drag);
    let dragging = Rc::clone(&state.dragging);
    let done = Rc::clone(&events.done);
    let listen = canvas(
        |_bounds, _window, _cx| {},
        move |_bounds, (), window, _cx| {
            let (panel, drag) = (Rc::clone(&panel), Rc::clone(&drag));
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && let Some(bounds) = panel.get()
                {
                    drag(asked(event.position.x, bounds, edge), window, cx);
                }
            });
            let (dragging, done) = (Rc::clone(&dragging), Rc::clone(&done));
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.button == MouseButton::Left
                    && dragging.replace(false)
                {
                    done(window, cx);
                    window.refresh();
                }
            });
        },
    )
    .size_full();
    let layer = popup::layer(div().w(size.width).h(size.height))
        .cursor_col_resize()
        .child(listen);
    Some(
        deferred(anchored().position(point(px(0.0), px(0.0))).child(layer))
            .with_priority(9)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{Edge, asked};

    /// The sidebar grows to the pointer; the code panel from its right
    /// edge back to the pointer.
    #[test]
    fn the_pointer_asks_for_the_width_from_the_fixed_edge() {
        let panel = Bounds::new(point(px(100.0), px(0.0)), size(px(300.0), px(800.0)));
        assert!((asked(px(450.0), panel, Edge::Right) - 350.0).abs() < 1e-4);
        assert!((asked(px(150.0), panel, Edge::Left) - 250.0).abs() < 1e-4);
    }
}
