//! A list row that opens a page beside the list (TRACKER 791), on a real
//! window with the timers on the test clock.
use super::super::*;
use crate::submenu::{Submenus, flyout};
use crate::{Scheme, Tokens};
use gpui::{Entity, Modifiers, Render, TestAppContext, VisualTestContext, div, point, px};
use std::time::Duration;

struct Probe {
    list: ComboboxState,
    page_list: ComboboxState,
    submenus: Submenus,
    chosen: usize,
    page_pressed: usize,
    inline_drawn: bool,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let rows = vec![
            ComboRow::new("Show ended"),
            ComboRow {
                page: Some("grouping".into()),
                opens: true,
                trailing: Some("Machine".into()),
                ..ComboRow::new("Grouping")
            },
        ];
        let header = ComboHeader {
            title: Some("Display".into()),
            back: false,
            search: false,
            empty: "Empty".into(),
            note: None,
            side: Side::Below,
        };
        let handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.list,
            chose: Rc::new(|this, _, _, _| this.chosen = this.chosen.saturating_add(1)),
            back: None,
        };
        let list = combobox(
            "display", &self.list, &header, &rows, &handlers, 232.0, &tokens, window, cx,
        );
        let page_rows = vec![ComboRow::new("By machine"), ComboRow::new("By status")];
        let page_header = ComboHeader {
            title: None,
            ..header.clone()
        };
        let page_handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.page_list,
            chose: Rc::new(|this, _, _, _| {
                this.page_pressed = this.page_pressed.saturating_add(1);
            }),
            back: None,
        };
        let inline = combobox_inline(
            "grouping",
            &self.page_list,
            &page_header,
            &page_rows,
            &page_handlers,
            232.0,
            &tokens,
            window,
            cx,
        );
        self.inline_drawn = inline.is_some();
        let page = (self.list.is_open() && self.submenus.is_open("grouping", 0))
            .then(|| inline.map(|inline| flyout(&self.submenus, "grouping", 0, 3, inline)))
            .flatten()
            .flatten();
        div().size_full().children(list).children(page)
    }
}

fn setup(cx: &mut TestAppContext, with_pages: bool) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        let initialized = crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok(), "faces and input globals register");
    });
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let submenus = Submenus::new();
        let mut list = ComboboxState::new(window, cx);
        list.set_submenus(with_pages.then_some(&submenus));
        Probe {
            list,
            page_list: ComboboxState::new(window, cx),
            submenus,
            chosen: 0,
            page_pressed: 0,
            inline_drawn: false,
        }
    });
    view.update_in(cx, |probe, window, cx| {
        probe.list.set_anchor(Some(Bounds {
            origin: point(px(40.0), px(40.0)),
            size: gpui::size(px(80.0), px(30.0)),
        }));
        probe.list.open(None, window, cx);
        probe.page_list.open(None, window, cx);
        probe.list.open(None, window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    (view, cx)
}

fn grouping_row(view: &Entity<Probe>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    view.read_with(cx, |probe, _| probe.submenus.trigger("grouping"))
        .unwrap()
}

fn state(view: &Entity<Probe>, cx: &mut VisualTestContext) -> (bool, bool, usize) {
    view.read_with(cx, |probe, _| {
        (
            probe.list.is_open(),
            probe.submenus.is_open("grouping", 0),
            probe.chosen,
        )
    })
}

#[gpui::test]
fn a_page_row_opens_its_page_on_rest_or_press_without_choosing(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = grouping_row(&view, cx);
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(90));
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
}

#[gpui::test]
fn a_press_on_the_page_keeps_the_list_and_one_outside_closes_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    let row = grouping_row(&view, cx);
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    let right = f32::from(row.right());
    let inside = point(px(40.0_f32.mul_add(1.0, right)), row.center().y);
    cx.simulate_click(inside, Modifiers::none());
    cx.run_until_parked();
    assert!(state(&view, cx).0);
    cx.simulate_click(point(px(10.0), px(300.0)), Modifiers::none());
    cx.run_until_parked();
    assert!(!state(&view, cx).0);
}

#[gpui::test]
fn enter_on_a_highlighted_page_row_opens_its_page(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, true, 0));
}

#[gpui::test]
fn without_submenus_the_screen_takes_the_choice(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, false);
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    assert_eq!(state(&view, cx), (true, false, 1));
}

#[gpui::test]
fn the_inline_list_draws_only_while_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, true);
    assert!(view.read_with(cx, |probe, _| probe.inline_drawn));
    view.update(cx, |probe, cx| {
        probe.page_list.close();
        cx.notify();
    });
    cx.run_until_parked();
    assert!(!view.read_with(cx, |probe, _| probe.inline_drawn));
}
