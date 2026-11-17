//! A row that opens a page (TRACKER 791): on a real window, with its page
//! drawn as the screen draws it.
use super::*;
use crate::submenu::{Submenus, flyout};
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{Entity, Modifiers, Render, TestAppContext, VisualTestContext, div, point, px};
use std::time::Duration;

struct Probe {
    menu: MenuState,
    submenus: Submenus,
    items: Vec<MenuItem>,
    chosen: usize,
    page_pressed: usize,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|this, _, _, _| this.chosen = this.chosen.saturating_add(1)),
        };
        let menu = menu_with(
            Content::new("test-menu", &self.items).with_submenus(&self.submenus),
            &self.menu,
            &handlers,
            &tokens,
            false,
            cx,
        );
        // The screen draws an open page after its menu, while the menu is open.
        let page = div()
            .id("page")
            .debug_selector(|| "page".to_owned())
            .w(px(180.0))
            .h(px(100.0))
            .on_click(cx.listener(|this: &mut Self, _, _, _| {
                this.page_pressed = this.page_pressed.saturating_add(1);
            }));
        let page = (self.menu.is_open() && self.submenus.is_open("labels", 0))
            .then(|| flyout(&self.submenus, "labels", 0, 1, page))
            .flatten();
        div().size_full().children(menu).children(page)
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|_, cx| Probe {
        menu: MenuState::new(cx),
        submenus: Submenus::new(),
        items: vec![
            MenuItem::new("Run"),
            MenuItem::sub("Labels", "labels").with_value("Ready"),
        ],
        chosen: 0,
        page_pressed: 0,
    });
    reopen(&view, cx);
    (view, cx)
}

fn reopen(view: &Entity<Probe>, cx: &mut VisualTestContext) {
    view.update_in(cx, |probe, window, cx| {
        probe.menu.open(point(px(100.0), px(40.0)), window, cx);
        cx.notify();
    });
    cx.run_until_parked();
}

fn state(view: &Entity<Probe>, cx: &mut VisualTestContext) -> (bool, bool, usize) {
    view.read_with(cx, |probe, _| {
        (
            probe.menu.is_open(),
            probe.submenus.is_open("labels", 0),
            probe.chosen,
        )
    })
}

fn sub_row_centre(view: &Entity<Probe>, cx: &mut VisualTestContext) -> gpui::Point<gpui::Pixels> {
    view.read_with(cx, |probe, _| probe.submenus.trigger("labels"))
        .unwrap()
        .center()
}

#[gpui::test]
fn a_sub_row_opens_its_page_on_rest_or_press_and_never_chooses(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let row = sub_row_centre(&view, cx);
    cx.simulate_mouse_move(row, None, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(90));
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
    cx.simulate_click(row, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
}

#[gpui::test]
fn a_press_on_the_page_keeps_the_menu_and_one_outside_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let row = sub_row_centre(&view, cx);
    cx.simulate_click(row, Modifiers::none());
    cx.run_until_parked();
    let page = cx.debug_bounds("page").unwrap();
    cx.simulate_click(page.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
    assert_eq!(view.read_with(cx, |probe, _| probe.page_pressed), 1);
    cx.simulate_click(point(px(20.0), px(300.0)), Modifiers::none());
    cx.run_until_parked();
    assert!(!state(&view, cx).0);
}

#[gpui::test]
fn enter_on_a_highlighted_sub_row_opens_its_page(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
}

#[gpui::test]
fn opening_the_menu_again_starts_at_its_root(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let row = sub_row_centre(&view, cx);
    cx.simulate_click(row, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!state(&view, cx).0);
    reopen(&view, cx);
    assert_eq!(state(&view, cx), (true, false, 0));
}
