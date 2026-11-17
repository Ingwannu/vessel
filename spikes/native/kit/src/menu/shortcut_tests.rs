//! A menu row's bound shortcut at its end, as Paseo's `DropdownMenuItem
//! trailing={<Shortcut chord>}` (TRACKER 776): pushed to the row's end, and
//! nothing drawn for a row without one.
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
            Content::new("split", &self.items).with_width(px(220.0)),
            &self.menu,
            &handlers,
            &Tokens::of(Scheme::Dark),
            false,
            cx,
        ))
    }
}

#[gpui::test]
fn a_bound_shortcut_ends_its_row_and_an_unbound_row_draws_none(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (_view, cx) = cx.add_window_view(|window, cx| {
        let mut menu = MenuState::new(cx);
        menu.open(point(px(100.0), px(40.0)), window, cx);
        Probe {
            menu,
            items: vec![
                MenuItem::new("Split pane right")
                    .with_leading(Glyph::Columns2)
                    .with_shortcut(vec!["Ctrl+\\".into()]),
                MenuItem::new("Split pane down").with_leading(Glyph::Rows2),
            ],
        }
    });
    cx.run_until_parked();
    let row = cx.debug_bounds("menu-row-0").unwrap();
    let keys = cx.debug_bounds("menu-row-shortcut-0").unwrap();
    // At the row's end, inside its 8-point inset and 1-point edge
    // (`marginLeft: auto`).
    let inset = f32::from(keys.right()).mul_add(-1.0, f32::from(row.right()));
    assert_eq!(inset.to_bits(), 9.0_f32.to_bits());
    assert!(keys.left() > row.left());
    assert!(cx.debug_bounds("menu-row-shortcut-1").is_none());
    assert!(cx.debug_bounds("menu-row-1").is_some());
}
