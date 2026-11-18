//! The list as a phone's sheet drags down to close, with the dialog's
//! engine (TRACKER 799): from its handle, or from its rows at their top.
use super::*;
use crate::{Scheme, Tokens};
use gpui::{
    Entity, Modifiers, MouseButton, Point, Render, TestAppContext, VisualTestContext, div, point, px,
    size,
};

struct Probe {
    list: ComboboxState,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let rows = vec![
            ComboRow::new("One"),
            ComboRow::new("Two"),
            ComboRow::new("Three"),
        ];
        let handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.list,
            chose: Rc::new(|_: &mut Self, _, _, _| {}),
            back: None,
        };
        let header = ComboHeader {
            title: Some("Pick".into()),
            ..crate::select::header()
        };
        let list = combobox(
            "drag-list",
            &self.list,
            &header,
            &rows,
            &handlers,
            240.0,
            &tokens,
            window,
            cx,
        );
        div().size_full().children(list)
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| Probe {
        list: ComboboxState::new(window, cx),
    });
    cx.simulate_resize(size(px(390.0), px(800.0)));
    view.update_in(cx, |probe, window, cx| {
        probe.list.open(None, window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    (view, cx)
}

/// The sheet's panel where it was last drawn.
fn panel(view: &Entity<Probe>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    view.read_with(cx, |probe, _| probe.list.sheet.panel_bounds())
        .unwrap()
}

/// Drag down by `distance` from `from`, then hold past the velocity window
/// before letting go, so only where it was let go decides.
fn drag_down(cx: &mut VisualTestContext, from: Point<Pixels>, distance: f32) {
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for step in 1_u8..=4 {
        let y = distance.mul_add(f32::from(step).mul_add(0.25, 0.0), f32::from(from.y));
        cx.simulate_mouse_move(point(from.x, px(y)), MouseButton::Left, Modifiers::none());
    }
    std::thread::sleep(std::time::Duration::from_millis(250));
    let to = point(from.x, px(f32::from(from.y).mul_add(1.0, distance)));
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

/// A point on the handle: centred, 6 below the panel's top.
fn on_handle(panel: Bounds<Pixels>) -> Point<Pixels> {
    point(
        panel.center().x,
        px(f32::from(panel.top()).mul_add(1.0, 6.0)),
    )
}

#[gpui::test]
fn its_handle_pulled_past_the_middle_closes_the_list(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let at = on_handle(panel(&view, cx));
    drag_down(cx, at, 600.0);
    view.read_with(cx, |probe, _| assert!(!probe.list.is_open()));
}

#[gpui::test]
fn a_short_pull_lets_it_spring_back_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let at = on_handle(panel(&view, cx));
    drag_down(cx, at, 10.0);
    view.read_with(cx, |probe, _| assert!(probe.list.is_open()));
}

#[gpui::test]
fn its_rows_pulled_down_from_their_top_close_it_too(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = panel(&view, cx);
    // Near the bottom: the last row, under the title.
    let at = point(
        panel.center().x,
        px(f32::from(panel.bottom()).mul_add(1.0, -30.0)),
    );
    drag_down(cx, at, 600.0);
    view.read_with(cx, |probe, _| assert!(!probe.list.is_open()));
}
