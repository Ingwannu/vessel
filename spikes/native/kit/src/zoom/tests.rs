//! The vendored GPUI's per-window zoom, through the test window's platform
//! callbacks (a resize, a press): the layout space shrinks by the zoom, the
//! paint scale grows by it, and a press from the platform lands on the
//! element drawn under it.
use gpui::{
    Context, Entity, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
    ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window, div, point,
    px, size,
};

/// A window whose one target, 100 points square at the origin, records
/// a press.
struct Target {
    pressed: bool,
}

impl Render for Target {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let target = div().size(px(100.0)).on_mouse_down(
            MouseButton::Left,
            cx.listener(|target, _event, _window, _cx| target.pressed = true),
        );
        div().size_full().child(target)
    }
}

/// A test window of 1000 by 800 platform points on a display of scale 2
/// (the test platform's), zoomed by `zoom`.
fn zoomed_window(cx: &mut TestAppContext, zoom: f32) -> (Entity<Target>, &mut VisualTestContext) {
    let (target, cx) = cx.add_window_view(|_, _| Target { pressed: false });
    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, size(px(1000.0), px(800.0)));
    cx.update(|window, cx| window.set_zoom(zoom, cx));
    cx.run_until_parked();
    (target, cx)
}

#[gpui::test]
fn the_zoom_divides_the_layout_space_and_multiplies_the_paint_scale(cx: &mut TestAppContext) {
    let (_target, cx) = zoomed_window(cx, 1.25);
    cx.update(|window, _| {
        assert_eq!(window.viewport_size(), size(px(800.0), px(640.0)));
        assert_eq!(window.scale_factor().to_bits(), 2.5_f32.to_bits());
        // The window itself keeps its size on the screen.
        assert_eq!(window.bounds().size, size(px(1000.0), px(800.0)));
    });
}

#[gpui::test]
fn a_platform_resize_keeps_the_zoom(cx: &mut TestAppContext) {
    let (_target, cx) = zoomed_window(cx, 2.0);
    let handle = cx.update(|window, _| window.window_handle());
    cx.simulate_window_resize(handle, size(px(600.0), px(400.0)));
    cx.update(|window, _| {
        assert_eq!(window.zoom().to_bits(), 2.0_f32.to_bits());
        assert_eq!(window.viewport_size(), size(px(300.0), px(200.0)));
    });
}

#[gpui::test]
fn a_press_lands_on_the_element_drawn_under_it_when_zoomed(cx: &mut TestAppContext) {
    // 150 platform points is 75 layout points at zoom 2: inside the target,
    // which is drawn over the first 200 platform points.
    let (target, cx) = zoomed_window(cx, 2.0);
    let press = point(px(150.0), px(150.0));
    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    assert!(cx.update(|_, cx| target.read(cx).pressed));
}

#[gpui::test]
fn unzoomed_the_same_press_misses(cx: &mut TestAppContext) {
    let (target, cx) = zoomed_window(cx, 1.0);
    let press = point(px(150.0), px(150.0));
    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    assert!(!cx.update(|_, cx| target.read(cx).pressed));
}

#[gpui::test]
fn the_zoom_stays_within_its_bounds_and_ignores_a_non_number(cx: &mut TestAppContext) {
    let (_target, cx) = zoomed_window(cx, 9.0);
    cx.update(|window, cx| {
        assert_eq!(window.zoom().to_bits(), Window::MAX_ZOOM.to_bits());
        window.set_zoom(f32::NAN, cx);
        assert_eq!(window.zoom().to_bits(), Window::MAX_ZOOM.to_bits());
        window.set_zoom(0.0, cx);
        assert_eq!(window.zoom().to_bits(), Window::MIN_ZOOM.to_bits());
    });
}
