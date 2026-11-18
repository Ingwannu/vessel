//! An ordinary row's hint at its end, muted, as Paseo's `hint` (TRACKER
//! 789): Copy agent id with the id's first seven characters; nothing drawn
//! for a row without one.
use super::*;
use crate::{Scheme, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Render, TestAppContext, div, point, px};

struct Probe {
    menu: MenuState,
    items: Vec<MenuItem>,
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
            false,
            cx,
        ))
    }
}

#[gpui::test]
fn a_rows_hint_ends_it_and_a_row_without_one_draws_none(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (_view, cx) = cx.add_window_view(|window, cx| {
        let mut menu = MenuState::new(cx);
        menu.open(point(px(100.0), px(40.0)), window, cx);
        Probe {
            menu,
            items: vec![
                MenuItem::new("Copy agent id")
                    .with_leading(Glyph::Copy)
                    .with_value("0a23934"),
                MenuItem::new("Rename").with_leading(Glyph::Pencil),
            ],
        }
    });
    cx.run_until_parked();
    let row = cx.debug_bounds("menu-row-0").unwrap();
    let hint = cx.debug_bounds("menu-row-hint-0").unwrap();
    // At the row's end, inside its 8-point inset and 1-point edge.
    let inset = f32::from(hint.right()).mul_add(-1.0, f32::from(row.right()));
    assert_eq!(inset.to_bits(), 9.0_f32.to_bits());
    assert!(hint.left() > row.left());
    assert!(cx.debug_bounds("menu-row-hint-1").is_none());
}
