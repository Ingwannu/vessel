//! A menu row's tooltip (TRACKER 803): Paseo's `menu-item.tsx` row
//! tooltip, right of the row and centred on it, 10 away, after 250 ms;
//! none for a row without words, none on a phone.
use super::*;
use crate::{Scheme, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Modifiers, Render, TestAppContext, VisualTestContext, div, point, px};
use std::time::Duration;

struct Probe {
    menu: MenuState,
    items: Vec<MenuItem>,
    narrow: bool,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|_, _, _, _| {}),
        };
        div().size_full().children(menu_with(
            Content::new("tab", &self.items).with_width(px(220.0)),
            &self.menu,
            &handlers,
            &Tokens::of(Scheme::Dark),
            self.narrow,
            cx,
        ))
    }
}

const WORDS: &str = "Reload agent to update skills, MCPs or login status.";

fn setup(cx: &mut TestAppContext, narrow: bool) -> &mut VisualTestContext {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (_view, cx) = cx.add_window_view(move |window, cx| {
        let mut menu = MenuState::new(cx);
        menu.open(point(px(40.0), px(40.0)), window, cx);
        Probe {
            menu,
            items: vec![
                MenuItem::new("Close other tabs").with_leading(Glyph::CopyX),
                MenuItem::new("Reload agent")
                    .with_leading(Glyph::RotateCw)
                    .with_tooltip(WORDS),
            ],
            narrow,
        }
    });
    cx.run_until_parked();
    cx
}

fn rest_on(cx: &mut VisualTestContext, row: &'static str) {
    let at = cx.debug_bounds(row).unwrap().center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.run_until_parked();
}

fn wait(cx: &mut VisualTestContext, millis: u64) {
    cx.executor().advance_clock(Duration::from_millis(millis));
    cx.run_until_parked();
}

#[gpui::test]
fn a_rows_tooltip_comes_after_its_delay_right_of_the_row_and_centred(cx: &mut TestAppContext) {
    let cx = setup(cx, false);
    rest_on(cx, "menu-row-1");
    wait(cx, 249);
    assert!(cx.debug_bounds("tooltip-card").is_none());
    wait(cx, 1);
    let card = cx.debug_bounds("tooltip-card").unwrap();
    let row = cx.debug_bounds("menu-row-1").unwrap();
    // Paseo's `side="right"`, `offset={10}`, `align="center"`.
    let gap = f32::from(row.right()).mul_add(-1.0, f32::from(card.left()));
    assert!((9.5..10.5).contains(&gap), "10 right of the row, was {gap}");
    let centre = f32::from(row.center().y).mul_add(-1.0, f32::from(card.center().y));
    assert!(
        (-0.5..0.5).contains(&centre),
        "centred on the row, off by {centre}"
    );
}

#[gpui::test]
fn a_row_without_words_shows_none(cx: &mut TestAppContext) {
    let cx = setup(cx, false);
    rest_on(cx, "menu-row-0");
    wait(cx, 500);
    assert!(cx.debug_bounds("tooltip-card").is_none());
}

#[gpui::test]
fn a_phones_menu_shows_none(cx: &mut TestAppContext) {
    let cx = setup(cx, true);
    rest_on(cx, "menu-row-1");
    wait(cx, 500);
    assert!(cx.debug_bounds("tooltip-card").is_none());
}
