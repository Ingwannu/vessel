//! Literal source rails and compact height through the same private row body.
use super::{Check, RowProfile};
use crate::menu::{MenuItem, item, row};
use crate::{Scheme, Tokens, popup::Surfaces};
use gpui::prelude::*;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window, div, px};
use std::{cell::RefCell, rc::Rc};
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    scheme: Scheme,
    profile: RowProfile,
    narrow: bool,
    check: Check,
    row: Seen,
    children: Seen,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let item = MenuItem::new("Option");
        let paint = item::paint(&item, false, &Surfaces::of(&tokens));
        let seen_row = Rc::clone(&self.row);
        let seen_children = Rc::clone(&self.children);
        div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| *seen_row.borrow_mut() = bounds)
            .child(
                row::frame(
                    0,
                    &item,
                    paint,
                    self.narrow,
                    &Surfaces::of(&tokens),
                    row::Content {
                        profile: self.profile,
                        check: self.check,
                        ..row::Content::default()
                    },
                )
                .on_children_prepainted(move |bounds, _, _| *seen_children.borrow_mut() = bounds)
                .id("source-row"),
            )
    }
}
#[gpui::test]
fn shared_menu_source_profile_and_reserved_check_rail_preserve_legacy_height(
    cx: &mut TestAppContext,
) {
    for scheme in [Scheme::Dark, Scheme::Light] {
        for (profile, narrow, height) in [
            (RowProfile::Legacy, false, 28.0),
            (RowProfile::Legacy, true, 44.0),
            (RowProfile::Reference, false, 28.0),
            (RowProfile::Reference, true, 40.0),
        ] {
            for (check, label_index, label_left) in [
                (Check::None, 0, 13.0),
                (Check::Leading(false), 1, 37.0),
                (Check::Leading(true), 1, 37.0),
            ] {
                let row = Seen::default();
                let children = Seen::default();
                let (_, cx) = cx.add_window_view({
                    let row = Rc::clone(&row);
                    let children = Rc::clone(&children);
                    move |_, _| Probe {
                        scheme,
                        profile,
                        narrow,
                        check,
                        row,
                        children,
                    }
                });
                cx.run_until_parked();
                let row = row.borrow();
                assert_eq!(row.len(), 1);
                let bounds = row.first().unwrap();
                assert_eq!(bounds.origin.x, px(4.0));
                assert_eq!(bounds.size.width, px(312.0));
                assert_eq!(bounds.size.height, px(height));
                let children = children.borrow();
                assert_eq!(children.get(label_index).unwrap().origin.x, px(label_left));
                if matches!(check, Check::Leading(_)) {
                    assert_eq!(children.first().unwrap().origin.x, px(13.0));
                    assert_eq!(children.first().unwrap().size.width, px(16.0));
                }
            }
        }
    }
}
