//! The menu as a phone's sheet (TRACKER 791, Paseo's `compactMode="sheet"`):
//! its place, its page replacing it in place, and its scrim.
use super::*;
use crate::submenu::Submenus;
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{
    Entity, FocusHandle, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext, div,
    point, px,
};

struct Probe {
    menu: MenuState,
    /// The opener: it holds the keys before the menu opens.
    root: FocusHandle,
    submenus: Submenus,
    items: Vec<MenuItem>,
    sheet: bool,
    drawn: bool,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|_, _, _, _| {}),
        };
        let content = Content::new("phone-menu", &self.items).with_submenus(&self.submenus);
        let content = if self.sheet {
            content.with_sheet("Actions")
        } else {
            content
        };
        let menu = menu_with(content, &self.menu, &handlers, &tokens, true, cx);
        self.drawn = menu.is_some();
        div().size_full().track_focus(&self.root).children(menu)
    }
}

fn setup(cx: &mut TestAppContext, sheet: bool) -> (Entity<Probe>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let root = cx.focus_handle();
        root.focus(window, cx);
        Probe {
            menu: MenuState::new(cx),
            root,
            submenus: Submenus::new(),
            items: vec![MenuItem::new("Pin"), MenuItem::sub("Labels", "labels")],
            sheet,
            drawn: false,
        }
    });
    view.update_in(cx, |probe, window, cx| {
        probe.menu.open(point(px(40.0), px(40.0)), window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    // The frame after the sheet's first measures it (no frame loop here).
    cx.update(|window, cx| {
        let _ran = window.simulate_next_frame(cx);
    });
    cx.run_until_parked();
    (view, cx)
}

fn labels_row(view: &Entity<Probe>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    view.read_with(cx, |probe, _| probe.submenus.trigger("labels"))
        .unwrap()
}

fn window_size(cx: &mut VisualTestContext) -> (f32, f32) {
    cx.update(|window, _| {
        let size = window.viewport_size();
        (f32::from(size.width), f32::from(size.height))
    })
}

#[gpui::test]
fn a_phone_menu_with_a_sheet_title_comes_up_from_the_bottom(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let (width, height) = window_size(cx);
    let row = labels_row(&view, cx);
    assert!(row.left() < px(16.0));
    assert!(row.right() > px(16.0_f32.mul_add(-1.0, width)));
    assert!(row.bottom() > px(height.mul_add(0.5, 0.0)));
}

#[gpui::test]
fn the_sheet_is_as_tall_as_its_rows(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = labels_row(&view, cx);
    let panel = view
        .read_with(cx, |probe, _| probe.menu.sheet.panel_bounds())
        .unwrap();
    assert!(panel.bottom() >= row.bottom());
}

#[gpui::test]
fn without_a_sheet_title_a_phone_keeps_the_popover(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    let (width, _) = window_size(cx);
    let row = labels_row(&view, cx);
    assert!(row.right() < px(16.0_f32.mul_add(-1.0, width)));
}

#[gpui::test]
fn its_page_replaces_it_and_back_brings_it_back(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = labels_row(&view, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        assert!(probe.menu.is_open());
        assert!(probe.submenus.is_open("labels", 0));
        assert!(!probe.drawn);
    });
    view.update(cx, |probe, cx| {
        assert!(probe.submenus.back());
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |probe, _| assert!(probe.drawn && probe.menu.is_open()));
}

#[gpui::test]
fn a_press_on_the_dimmed_page_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    cx.simulate_click(point(px(20.0), px(20.0)), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |probe, _| assert!(!probe.menu.is_open()));
}

#[gpui::test]
fn a_resting_pointer_does_not_open_a_page_of_the_sheet(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = labels_row(&view, cx);
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(300));
    cx.run_until_parked();
    view.read_with(cx, |probe, _| {
        assert!(!probe.submenus.is_open("labels", 0));
        assert!(probe.drawn);
    });
}

/// Drag from `from` down by `distance`, then hold past the velocity window
/// before letting go, so only where it was let go decides.
fn drag_down(cx: &mut VisualTestContext, from: Point<Pixels>, distance: f32) {
    let to = point(from.x, px(f32::from(from.y).mul_add(1.0, distance)));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for step in 1_u8..=4 {
        let y = distance.mul_add(f32::from(step).mul_add(0.25, 0.0), f32::from(from.y));
        cx.simulate_mouse_move(point(from.x, px(y)), MouseButton::Left, Modifiers::none());
    }
    std::thread::sleep(std::time::Duration::from_millis(250));
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn a_pan_down_the_sheet_past_its_middle_closes_the_menu(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = labels_row(&view, cx);
    let (_, height) = window_size(cx);
    drag_down(cx, row.center(), height);
    view.read_with(cx, |probe, _| assert!(!probe.menu.is_open()));
    // The keys go back to the opener, as an Escape would give them.
    cx.update(|window, cx| view.read_with(cx, |probe, _| assert!(probe.root.is_focused(window))));
}

#[gpui::test]
fn a_short_pan_lets_the_sheet_spring_back_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = labels_row(&view, cx);
    drag_down(cx, row.center(), 12.0);
    view.read_with(cx, |probe, _| {
        assert!(probe.menu.is_open());
        assert!(!probe.submenus.is_open("labels", 0));
    });
}
