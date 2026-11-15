//! Real native measurement/hit/input with host-supplied admission, no focus owner.
use super::*;
use gpui::prelude::*;
use gpui::{
    Bounds, Context, Entity, InputEvent as _, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, TestAppContext, VisualTestContext,
    deferred, div, point, px, size,
};
use std::cell::Cell;
#[derive(Clone, Copy)]
enum Contents {
    Handle,
    Scrolling,
}
struct Probe {
    contents: Contents,
    fit: State,
    natural: Pixels,
    generation: Rc<Cell<usize>>,
    renders: Rc<Cell<usize>>,
    measured: Rc<Cell<Option<Bounds<Pixels>>>>,
    clicks: Rc<Cell<usize>>,
    closed: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        let current = self.generation.get();
        let generation = Rc::clone(&self.generation);
        let closed = Rc::clone(&self.closed);
        let handlers = Handlers {
            admit: Rc::new(move |_, _| generation.get() == current),
            closed: Rc::new(move |_, _| closed.set(closed.get().saturating_add(1))),
        };
        let seen = Rc::clone(&self.measured);
        let clicks = Rc::clone(&self.clicks);
        let body = match self.contents {
            Contents::Handle => div()
                .size_full()
                .on_mouse_down(MouseButton::Left, move |_, _, _| {
                    clicks.set(clicks.get().saturating_add(1));
                })
                .into_any_element(),
            Contents::Scrolling => div()
                .id("fit-test-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(self.fit.scroll())
                .child(div().h(px(1200.0)).flex_none())
                .relative()
                .child(self.fit.input(Origin::Content, &handlers))
                .into_any_element(),
        };
        let panel = div()
            .h(self.natural)
            .flex_shrink(1.0)
            .min_h_0()
            .on_children_prepainted(move |bounds, _, _| seen.set(bounds.first().copied()))
            .flex()
            .flex_col()
            .child(body)
            .relative()
            .when(matches!(self.contents, Contents::Handle), |panel| {
                panel.child(self.fit.input(Origin::Handle, &handlers))
            });
        div()
            .size_full()
            .p(px(20.0))
            .child(deferred(self.fit.element(
                panel,
                Percentage::new(80).unwrap(),
                window,
                &handlers,
            )))
    }
}
fn setup(cx: &mut TestAppContext, height: f32) -> (Entity<Probe>, &mut VisualTestContext) {
    setup_with(cx, height, Contents::Handle)
}
fn setup_with(
    cx: &mut TestAppContext,
    height: f32,
    contents: Contents,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        contents,
        fit: State::new(),
        natural: px(height),
        generation: Rc::new(Cell::new(0)),
        renders: Rc::new(Cell::new(0)),
        measured: Rc::new(Cell::new(None)),
        clicks: Rc::new(Cell::new(0)),
        closed: Rc::new(Cell::new(0)),
    });
    cx.simulate_resize(size(px(400.0), px(600.0)));
    cx.run_until_parked();
    (view, cx)
}
fn down(window: &mut Window, cx: &mut App, position: Point<Pixels>) {
    let _result = window.dispatch_event(
        MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
}
fn motion(window: &mut Window, cx: &mut App, position: Point<Pixels>) {
    let _result = window.dispatch_event(
        MouseMoveEvent {
            position,
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
        }
        .to_platform_input(),
        cx,
    );
}
fn up(window: &mut Window, cx: &mut App, position: Point<Pixels>) {
    let _result = window.dispatch_event(
        MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
}
fn drag(window: &mut Window, cx: &mut App) {
    down(window, cx, point(px(100.0), px(305.0)));
    motion(window, cx, point(px(100.0), px(590.0)));
    up(window, cx, point(px(100.0), px(590.0)));
}
#[gpui::test]
fn fitted_frame_measures_fractional_content_and_caps_large_content_in_native_hit_bounds(
    cx: &mut TestAppContext,
) {
    for (natural, expected_top, expected_height) in [(173.5, 426.5, 173.5), (900.0, 120.0, 480.0)] {
        let (view, cx) = setup(cx, natural);
        let seen = view.read_with(cx, |this, _| this.measured.get().unwrap());
        assert_eq!(seen.origin.x, px(0.0));
        assert!(seen.origin.y.as_f32().mul_add(-1.0, expected_top).abs() < 0.001);
        assert_eq!(seen.size.width, px(400.0));
        assert!(
            seen.size
                .height
                .as_f32()
                .mul_add(-1.0, expected_height)
                .abs()
                < 0.001
        );
        cx.simulate_click(seen.center(), Modifiers::none());
        assert_eq!(view.read_with(cx, |this, _| this.clicks.get()), 1);
        cx.simulate_click(point(px(100.0), px(30.0)), Modifiers::none());
        assert_eq!(
            view.read_with(cx, |this, _| this.clicks.get()),
            1,
            "old parent origin is not hit"
        );
    }
}
#[gpui::test]
fn fitted_retained_frame_requires_the_host_admission_before_pointer_scroll_or_close(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, 300.0);
    cx.update(|window, cx| {
        let before = view.read(cx).renders.get();
        view.update(cx, |this, _| {
            this.generation.set(1);
            this.fit.reset();
        });
        drag(window, cx);
        let this = view.read(cx);
        assert_eq!(
            this.renders.get(),
            before,
            "dispatch retained A before B redraw"
        );
        assert_eq!(this.closed.get(), 0, "A cannot request B dismissal");
        assert_eq!(this.fit.scroll.offset(), point(px(0.0), px(0.0)));
        assert!(
            matches!(
                this.fit
                    .motion
                    .borrow_mut()
                    .frame(px(600.0), std::time::Instant::now(), true),
                snap::Frame::Closed
            ),
            "A cannot initialize or drag B geometry"
        );
    });
    cx.run_until_parked();
    cx.update(|window, cx| drag(window, cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.closed.get()),
        1,
        "fresh B pan still works"
    );
    cx.run_until_parked();
    let seen = view.read_with(cx, |this, _| this.measured.get().unwrap());
    assert!(
        seen.origin.y.as_f32().mul_add(-1.0, 300.0).abs() < 0.001,
        "refused unit close retains the same fitted open position"
    );
}

#[gpui::test]
fn fitted_native_body_scrolls_before_pulling_the_same_sheet_down(cx: &mut TestAppContext) {
    let (view, cx) = setup_with(cx, 300.0, Contents::Scrolling);
    for (start, end, expected_scroll) in [(500.0, 400.0, 100.0), (400.0, 450.0, 50.0)] {
        cx.update(|window, cx| {
            down(window, cx, point(px(100.0), px(start)));
            motion(window, cx, point(px(100.0), px(end)));
            up(window, cx, point(px(100.0), px(end)));
        });
        cx.run_until_parked();
        view.read_with(cx, |this, _| {
            assert!(
                this.fit
                    .scroll
                    .offset()
                    .y
                    .as_f32()
                    .mul_add(1.0, expected_scroll)
                    .abs()
                    < 0.001
            );
            assert!(
                this.measured
                    .get()
                    .unwrap()
                    .origin
                    .y
                    .as_f32()
                    .mul_add(-1.0, 300.0)
                    .abs()
                    < 0.001
            );
            assert_eq!(
                this.closed.get(),
                0,
                "scrolling rows cannot dismiss the sheet"
            );
        });
    }
    cx.update(|window, cx| {
        down(window, cx, point(px(100.0), px(305.0)));
        motion(window, cx, point(px(100.0), px(590.0)));
        up(window, cx, point(px(100.0), px(590.0)));
    });
    cx.run_until_parked();
    view.read_with(cx, |this, _| {
        assert_eq!(this.fit.scroll.offset().y, px(0.0));
        assert_eq!(
            this.closed.get(),
            1,
            "after content reaches top, the pan may close"
        );
    });
}
