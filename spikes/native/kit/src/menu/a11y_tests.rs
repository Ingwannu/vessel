//! Menu rows in GPUI's real accessibility tree (TRACKER 808): Paseo's
//! `menuitem` named by its text (the words shown, then the hint), a row
//! that cannot be chosen offering no Click, a page's row saying whether its
//! page is open.
use super::*;
use crate::a11y_snapshot::{A11yNode, activate, nodes};
use crate::submenu::Submenus;
use crate::{Scheme, Tokens};
use gpui::prelude::*;
use gpui::{Render, TestAppContext, VisualTestContext, div, point, px};

struct Probe {
    menu: MenuState,
    items: Vec<MenuItem>,
    pages: Submenus,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|_, _, _, _| {}),
        };
        let content = Content::new("tab", &self.items)
            .with_width(px(220.0))
            .with_submenus(&self.pages);
        div().size_full().children(menu_with(
            content,
            &self.menu,
            &handlers,
            &Tokens::of(Scheme::Dark),
            false,
            cx,
        ))
    }
}

fn items() -> Vec<MenuItem> {
    let mut closed = MenuItem::new("Close other tabs");
    closed.disabled = true;
    vec![
        MenuItem::new("Copy agent id").with_value("16aa40f"),
        closed,
        MenuItem::new("Create").pending("Creating…"),
        MenuItem::sub("Labels", "labels"),
    ]
}

fn setup(cx: &mut TestAppContext) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut menu = MenuState::new(cx);
        menu.open(point(px(40.0), px(40.0)), window, cx);
        Probe {
            menu,
            items: items(),
            pages: Submenus::new(),
        }
    });
    cx.run_until_parked();
    activate(cx);
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    (view, cx)
}

fn menu_items(cx: &mut VisualTestContext) -> Vec<A11yNode> {
    cx.update(|window, _| nodes(window, "MenuItem").unwrap())
}

fn named<'a>(nodes: &'a [A11yNode], name: &str) -> &'a A11yNode {
    nodes
        .iter()
        .find(|node| node.label.as_deref() == Some(name))
        .unwrap()
}

#[gpui::test]
fn every_row_is_a_menu_item_named_by_its_words_and_hint(cx: &mut TestAppContext) {
    let (_view, cx) = setup(cx);
    let rows = menu_items(cx);
    let names: Vec<Option<&str>> = rows.iter().map(|row| row.label.as_deref()).collect();
    assert_eq!(
        names,
        [
            Some("Close other tabs"),
            Some("Copy agent id 16aa40f"),
            Some("Creating…"),
            Some("Labels"),
        ]
    );
    // A row that can be chosen takes a click; a disabled or pending one
    // offers none.
    let clicks = |name: &str| {
        named(&rows, name)
            .actions
            .iter()
            .any(|action| action == "Click")
    };
    assert!(clicks("Copy agent id 16aa40f"));
    assert!(clicks("Labels"));
    assert!(!clicks("Close other tabs"));
    assert!(!clicks("Creating…"));
    // Plain rows carry no checked state (Paseo's rows without a check).
    assert!(rows.iter().all(|row| row.toggled.is_none()));
}

#[gpui::test]
fn a_pages_row_says_whether_its_page_is_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    assert_eq!(named(&menu_items(cx), "Labels").expanded, Some(false));
    view.update(cx, |probe, cx| {
        probe.pages.open("labels", 0);
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(named(&menu_items(cx), "Labels").expanded, Some(true));
    // Rows that open no page say nothing of it.
    assert_eq!(
        named(&menu_items(cx), "Copy agent id 16aa40f").expanded,
        None
    );
}
