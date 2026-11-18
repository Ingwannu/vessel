//! The menu as a phone's sheet (TRACKER 791, Paseo's `compactMode="sheet"`):
//! its place, its page replacing it in place, and its scrim.
use super::*;
use crate::submenu::Submenus;
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{Entity, Modifiers, Render, TestAppContext, VisualTestContext, div, point, px};

struct Probe {
    menu: MenuState,
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
        div().size_full().children(menu)
    }
}

fn setup(cx: &mut TestAppContext, sheet: bool) -> (Entity<Probe>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |_, cx| Probe {
        menu: MenuState::new(cx),
        submenus: Submenus::new(),
        items: vec![MenuItem::new("Pin"), MenuItem::sub("Labels", "labels")],
        sheet,
        drawn: false,
    });
    view.update_in(cx, |probe, window, cx| {
        probe.menu.open(point(px(40.0), px(40.0)), window, cx);
        cx.notify();
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
