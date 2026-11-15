//! Real keyboard edits, including a retry with retained selection.
use super::*;
use crate::{Scheme, SchemeChoice, Tokens};
use gpui::{Context, IntoElement, ParentElement as _, Render, TestAppContext, Window, div};

struct Probe {
    field: TextField,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(self.field.element(&Tokens::of(Scheme::Dark), false, cx))
    }
}

#[gpui::test]
fn a_waiting_form_keeps_text_focus_and_selection_until_retry(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for multiline in [false, true] {
        let (view, cx) = cx.add_window_view(move |window, cx| Probe {
            field: if multiline {
                TextField::form_multiline("", |_, _, _, _| {}, window, cx)
            } else {
                TextField::new("", |_, _, _, _| {}, window, cx)
            },
        });
        cx.update(|window, _cx| window.activate_window());
        view.update_in(cx, |view, window, cx| view.field.focus(window, cx));
        cx.run_until_parked();
        cx.simulate_input("answer ABC");
        cx.simulate_keystrokes("left shift-left");
        view.update_in(cx, |view, window, cx| {
            view.field.set_editability(Editability::ReadOnly, cx);
            assert!(view.field.focus_handle(cx).is_focused(window));
        });
        cx.run_until_parked();
        cx.simulate_input("discarded");
        view.update_in(cx, |view, window, cx| {
            assert_eq!(view.field.text(cx), "answer ABC");
            assert!(view.field.focus_handle(cx).is_focused(window));
            view.field.set_editability(Editability::Editable, cx);
        });
        cx.run_until_parked();
        cx.simulate_input("X");
        view.update_in(cx, |view, _window, cx| {
            assert_eq!(view.field.text(cx), "answer AXC");
        });
    }
}
