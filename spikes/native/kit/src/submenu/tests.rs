//! Hover intent and flyout placement on a real window, with the timers on
//! the test clock.
use super::{EDGE, OVERLAP, Submenus, flyout};
use gpui::prelude::*;
use gpui::{
    Bounds, Context, Entity, Modifiers, Pixels, Point, TestAppContext, VisualTestContext, Window,
    div, point, px,
};
use std::rc::Rc;
use std::time::Duration;

/// A trigger row ("show") and, while its page is open, the page's flyout.
struct Probe {
    submenus: Submenus,
    /// The row sits at the window's right edge.
    at_edge: bool,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width);
        let left = if self.at_edge {
            70.0_f32.mul_add(-1.0, width)
        } else {
            40.0
        };
        let hover = self.submenus.clone();
        let row = div()
            .id("row")
            .debug_selector(|| "row".to_owned())
            .absolute()
            .left(px(left))
            .top(px(40.0))
            .w(px(60.0))
            .h(px(28.0))
            .on_hover(move |inside: &bool, window, cx| {
                if *inside {
                    hover.hover_open("show", 0, window, cx);
                } else {
                    hover.hover_close(0, window, cx);
                }
            })
            .child(self.submenus.trigger_probe("show"));
        let page = div()
            .debug_selector(|| "page".to_owned())
            .w(px(180.0))
            .h(px(120.0));
        let flyout = self
            .submenus
            .is_open("show", 0)
            .then(|| flyout(&self.submenus, "show", 0, 1, page))
            .flatten();
        div().size_full().relative().child(row).children(flyout)
    }
}

fn setup(cx: &mut TestAppContext, at_edge: bool) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.add_window_view(move |_, _| Probe {
        submenus: Submenus::new(),
        at_edge,
    })
}

fn advance(cx: &VisualTestContext, millis: u64) {
    cx.executor().advance_clock(Duration::from_millis(millis));
    cx.run_until_parked();
}

fn pointer(cx: &mut VisualTestContext, at: Point<Pixels>) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.run_until_parked();
}

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(selector).unwrap()
}

fn open(view: &Entity<Probe>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |probe, _| probe.submenus.is_open("show", 0))
}

fn nowhere() -> Point<Pixels> {
    point(px(4.0), px(220.0))
}

#[gpui::test]
fn a_page_opens_when_the_pointer_rests_and_overlaps_its_menu(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    let row = bounds(cx, "row");
    pointer(cx, row.center());
    advance(cx, 89);
    assert!(!open(&view, cx));
    advance(cx, 1);
    assert!(open(&view, cx));
    let page = bounds(cx, "page");
    assert_eq!(
        page.origin,
        point(px(OVERLAP.mul_add(-1.0, f32::from(row.right()))), row.top())
    );
    // A press on the page is not outside the menu; one elsewhere is.
    view.read_with(cx, |probe, _| {
        assert!(probe.submenus.contains(page.center()));
        assert!(!probe.submenus.contains(nowhere()));
    });
}

#[gpui::test]
fn leaving_closes_after_the_grace_unless_the_pointer_reaches_the_page(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    let row = bounds(cx, "row");
    pointer(cx, row.center());
    advance(cx, 90);
    pointer(cx, nowhere());
    advance(cx, 259);
    assert!(open(&view, cx));
    advance(cx, 1);
    assert!(!open(&view, cx));
    // Into the page within the grace: it stays; leaving the page closes it.
    pointer(cx, row.center());
    advance(cx, 90);
    let page = bounds(cx, "page");
    pointer(cx, nowhere());
    advance(cx, 100);
    pointer(cx, page.center());
    advance(cx, 400);
    assert!(open(&view, cx));
    pointer(cx, nowhere());
    advance(cx, 260);
    assert!(!open(&view, cx));
}

#[gpui::test]
fn a_press_only_page_opens_on_press_and_hover_does_not_close_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    view.update(cx, |probe, _| probe.submenus.press_only("show"));
    let row = bounds(cx, "row");
    pointer(cx, row.center());
    advance(cx, 300);
    assert!(!open(&view, cx));
    view.update(cx, |probe, cx| {
        probe.submenus.open("show", 0);
        cx.notify();
    });
    cx.run_until_parked();
    pointer(cx, nowhere());
    advance(cx, 600);
    assert!(open(&view, cx));
}

#[gpui::test]
fn a_page_at_the_windows_edge_stays_inside_without_flipping(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    view.update(cx, |probe, cx| {
        probe.submenus.open("show", 0);
        cx.notify();
    });
    cx.run_until_parked();
    let row = bounds(cx, "row");
    let page = bounds(cx, "page");
    let width = cx.update(|window, _| f32::from(window.viewport_size().width));
    assert!(page.right() <= px(EDGE.mul_add(-1.0, width)));
    // Flipped, its right edge would meet the row's left; kept on the right
    // side, it reaches past it.
    assert!(page.right() > row.left());
    assert_eq!(page.top(), row.top());
}

#[gpui::test]
fn a_new_opening_of_the_menu_starts_at_the_root(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    let first = Rc::new(());
    let second = Rc::new(());
    view.update(cx, |probe, _| {
        probe.submenus.follow(&Rc::downgrade(&first));
        probe.submenus.open("show", 0);
        probe.submenus.follow(&Rc::downgrade(&first));
        assert!(probe.submenus.is_open("show", 0));
        probe.submenus.follow(&Rc::downgrade(&second));
        assert!(!probe.submenus.is_open("show", 0));
    });
}

#[gpui::test]
fn a_press_asks_its_screen_once_to_move_the_keys_in_and_hover_never(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    let row = bounds(cx, "row");
    pointer(cx, row.center());
    advance(cx, 90);
    view.update(cx, |probe, _| {
        assert!(probe.submenus.is_open("show", 0));
        assert!(!probe.submenus.pressed("show"));
        probe.submenus.open("show", 0);
        assert!(probe.submenus.pressed("show"));
        assert!(probe.submenus.take_pressed("show"));
        assert!(!probe.submenus.take_pressed("show"));
        // Closed before its screen took the keys: nothing left to take.
        probe.submenus.open("show", 0);
        assert!(probe.submenus.back());
        assert!(!probe.submenus.back());
        assert!(!probe.submenus.pressed("show"));
    });
}
