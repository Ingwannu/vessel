//! A list that is itself a menu's page (TRACKER 792: a row menu's Labels
//! list): its opening leaves the menu's path alone, and its row's page opens
//! after the list's own, a press-only one on a press alone.
use super::super::*;
use crate::submenu::Submenus;
use crate::{Scheme, Tokens};
use gpui::{Entity, Modifiers, Render, TestAppContext, VisualTestContext, div, px};
use std::time::Duration;

struct Probe {
    list: ComboboxState,
    submenus: Submenus,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let rows = vec![
            ComboRow::new("Ready"),
            ComboRow {
                page: Some("create".into()),
                opens: true,
                ..ComboRow::new("Create label")
            },
        ];
        let header = ComboHeader {
            title: None,
            back: false,
            search: false,
            empty: "Empty".into(),
            note: None,
            side: Side::Below,
        };
        let handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.list,
            chose: Rc::new(|_, _, _, _| {}),
            back: None,
        };
        let list = combobox_inline(
            "labels", &self.list, &header, &rows, &handlers, 232.0, &tokens, window, cx,
        );
        div().size_full().pt(px(40.0)).children(list)
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        let initialized = crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok(), "faces and input globals register");
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let submenus = Submenus::new();
        // The menu's Labels page is open; the list is that page.
        submenus.open("labels", 0);
        submenus.press_only("create");
        let mut list = ComboboxState::new(window, cx);
        list.set_nested_submenus(Some(&submenus), 1);
        Probe { list, submenus }
    });
    view.update_in(cx, |probe, window, cx| {
        probe.list.open_beside(None, window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    (view, cx)
}

fn path(view: &Entity<Probe>, cx: &mut VisualTestContext) -> (bool, bool) {
    view.read_with(cx, |probe, _| {
        (
            probe.submenus.is_open("labels", 0),
            probe.submenus.is_open("create", 1),
        )
    })
}

#[gpui::test]
fn the_lists_opening_keeps_its_menus_page(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    assert_eq!(path(&view, cx), (true, false));
}

#[gpui::test]
fn a_press_opens_the_rows_page_after_the_lists_and_rest_does_not(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let row = view
        .read_with(cx, |probe, _| probe.submenus.trigger("create"))
        .unwrap();
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(300));
    cx.run_until_parked();
    assert_eq!(path(&view, cx), (true, false));
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(path(&view, cx), (true, true));
}
