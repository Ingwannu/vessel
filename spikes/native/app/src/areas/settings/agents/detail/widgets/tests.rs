//! Scroll access is independent of clipboard correctness: the whole value
//! must contribute its width, and a wheel over it must leave the page still.
use super::*;
use gpui::{
    Bounds, Context, IntoElement, Pixels, Render, ScrollDelta, ScrollEvent, ScrollHandle,
    TestAppContext, Window, point,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vessel_app_kit::theme::{Scheme, Tokens};

struct Probe {
    handle: ScrollHandle,
    parent: Rc<Cell<usize>>,
    seen: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let text = "a very long command argument ".repeat(40);
        let width = fonts::measure(window, &text, Type::Mono);
        let parent = Rc::clone(&self.parent);
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(390.0))
            .flex()
            .flex_col()
            .on_scroll(move |_, _, _| parent.set(parent.get().saturating_add(1)))
            .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
            .child(value(&text, width, &tokens).track_scroll(&self.handle))
            .child(command_box("command", &text, &tokens, window, cx))
    }
}

#[gpui::test]
fn long_command_scrolls_without_moving_the_settings_page(cx: &mut TestAppContext) {
    let parent = Rc::new(Cell::new(0));
    let handle = ScrollHandle::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (_, cx) = cx.add_window_view({
        let parent = Rc::clone(&parent);
        let handle = handle.clone();
        let seen = Rc::clone(&seen);
        move |_, _| Probe {
            handle,
            parent,
            seen,
        }
    });
    cx.run_until_parked();
    let bounds = seen.borrow().clone();
    assert_eq!(bounds.len(), 2);
    assert!(
        handle.max_offset().x > px(390.0),
        "the value has no horizontal scroll extent"
    );
    cx.simulate_event(ScrollEvent {
        position: bounds[0].center(),
        delta: ScrollDelta::Lines(point(-3.0, 0.0)),
        ..ScrollEvent::default()
    });
    cx.run_until_parked();
    assert!(handle.offset().x < px(0.0), "the long value did not move");
    let before = parent.get();
    assert!(before > 0, "the page probe never received a scroll event");
    cx.simulate_event(ScrollEvent {
        position: bounds[1].center(),
        delta: ScrollDelta::Lines(point(-3.0, 0.0)),
        ..ScrollEvent::default()
    });
    cx.run_until_parked();
    assert_eq!(
        parent.get(),
        before,
        "a command wheel also reached the page"
    );
}
