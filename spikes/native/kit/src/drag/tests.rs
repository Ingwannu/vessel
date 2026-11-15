//! The vendored GPUI's per-element drag distance, through real mouse
//! events: an element that names one waits for it, one that does not
//! keeps GPUI's 2 points.
use gpui::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton, Pixels,
    Render, StatefulInteractiveElement as _, Styled as _, TestAppContext, Window, div, point, px,
};

use super::LIST_DRAG_DISTANCE;

struct Dragged;

impl Render for Dragged {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        gpui::Empty
    }
}

struct Row {
    distance: Option<Pixels>,
}

impl Render for Row {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let row = div()
            .id("row")
            .size(px(200.0))
            .on_drag((), |(), _offset, _window, cx| cx.new(|_| Dragged));
        match self.distance {
            Some(distance) => row.drag_threshold(distance),
            None => row,
        }
    }
}

/// Whether a drag started after a press at (20, 20) moved right to `x`
/// with the button held.
fn dragging_after(cx: &mut TestAppContext, distance: Option<Pixels>, x: f32) -> bool {
    let (_row, cx) = cx.add_window_view(|_, _| Row { distance });
    cx.run_until_parked();
    let start = point(px(20.0), px(20.0));
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(point(px(x), px(20.0)), MouseButton::Left, Modifiers::none());
    cx.update(|_window, cx| cx.has_active_drag())
}

#[gpui::test]
fn a_list_row_waits_for_paseos_six_points(cx: &mut TestAppContext) {
    // 5 points from the press, then 7.
    assert!(!dragging_after(cx, Some(LIST_DRAG_DISTANCE), 25.0));
    assert!(dragging_after(cx, Some(LIST_DRAG_DISTANCE), 27.0));
}

#[gpui::test]
fn an_element_naming_no_distance_keeps_gpuis_two(cx: &mut TestAppContext) {
    // 1.5 points from the press, then 3.
    assert!(!dragging_after(cx, None, 21.5));
    assert!(dragging_after(cx, None, 23.0));
}
