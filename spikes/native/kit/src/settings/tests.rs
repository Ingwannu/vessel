//! The optional slot must preserve existing sections and align an action
//! to the title's head rather than adding another row between it and card.
use super::*;
use crate::theme::Scheme;
use gpui::{Bounds, Context, IntoElement, Pixels, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Copy)]
enum Kind {
    Legacy,
    EmptySlot,
    Action,
}
struct Probe {
    kind: Kind,
    seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    action: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let card = div().h(px(80.0));
        let action = Rc::clone(&self.action);
        let trailing = div()
            .size(px(28.0))
            .on_children_prepainted(move |bounds, _, _| *action.borrow_mut() = bounds)
            .child(div().size_full())
            .into_any_element();
        let section = match self.kind {
            Kind::Legacy => section("Agents", card, &tokens),
            Kind::EmptySlot => section_with_trailing("Agents", card, None, &tokens),
            Kind::Action => section_with_trailing("Agents", card, Some(trailing), &tokens),
        };
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(360.0))
            .child(section.on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds))
    }
}
fn measure(cx: &mut TestAppContext, kind: Kind) -> (Vec<Bounds<Pixels>>, Vec<Bounds<Pixels>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let action = Rc::new(RefCell::new(Vec::new()));
    let (_, cx) = cx.add_window_view({
        let seen = Rc::clone(&seen);
        let action = Rc::clone(&action);
        move |_, _| Probe { kind, seen, action }
    });
    cx.run_until_parked();
    let bounds = seen.borrow().clone();
    let action_bounds = action.borrow().clone();
    (bounds, action_bounds)
}
#[gpui::test]
fn empty_slot_preserves_geometry_and_action_is_centered_at_the_trailing_edge(
    cx: &mut TestAppContext,
) {
    let (legacy, _) = measure(cx, Kind::Legacy);
    let (empty, _) = measure(cx, Kind::EmptySlot);
    assert_eq!(legacy, empty);
    assert_eq!(legacy.len(), 2);
    assert_eq!(legacy[0].left(), px(4.0));
    assert_eq!(legacy[1].left(), px(0.0));
    let (with_action, action) = measure(cx, Kind::Action);
    assert_eq!(with_action.len(), 2);
    assert_eq!(action.len(), 1);
    assert_eq!(action[0].right(), with_action[0].right());
    assert_eq!(action[0].center().y, with_action[0].center().y);
    assert_eq!(with_action[0].top(), empty[0].top());
    assert_eq!(with_action[0].left(), empty[0].left());
    assert_eq!(
        with_action[1].top(),
        px(f32::from(with_action[0].bottom()).mul_add(1.0, 12.0))
    );
    assert_eq!(
        empty[1].top(),
        px(f32::from(empty[0].bottom()).mul_add(1.0, 12.0))
    );
}
