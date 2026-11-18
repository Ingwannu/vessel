//! The rename field's character limit (Paseo's `maxLength`).
use super::*;
use crate::{Scheme, SchemeChoice, Tokens};
use gpui::{Context, IntoElement, ParentElement as _, Render, TestAppContext, Window, div};

struct Probe {
    field: TextField,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(self.field.rename_element(&Tokens::of(Scheme::Dark), cx))
    }
}

#[gpui::test]
fn the_field_takes_no_character_past_its_limit(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|window, cx| Probe {
        field: TextField::new("", |_, _, _, _| {}, window, cx),
    });
    cx.update(|window, _cx| window.activate_window());
    view.update_in(cx, |view, window, cx| {
        view.field.set_max_chars(5, cx);
        view.field.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("abcd");
    cx.simulate_input("é");
    cx.simulate_input("f");
    view.update_in(cx, |view, _window, cx| {
        // Characters, not bytes: "é" is the fifth; "f" would be the sixth.
        assert_eq!(view.field.text(cx), "abcdé");
    });
    cx.simulate_keystrokes("backspace");
    cx.simulate_input("g");
    view.update_in(cx, |view, _window, cx| {
        assert_eq!(view.field.text(cx), "abcdg");
    });
}
