//! The real editor emits one shared callback stream; legacy defaults stay quiet.
use super::*;
use crate::{Scheme, TextField, Tokens};
use gpui::prelude::*;
use gpui::{Render, TestAppContext, div};

#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Focus,
    Edit(String),
    Submit(String),
    Blur(String),
    Legacy(String),
}
struct Probe {
    field: TextField,
    legacy: TextField,
    other: TextField,
    events: Vec<Seen>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div()
            .flex()
            .flex_col()
            .child(self.field.element(&tokens, false, cx))
            .child(self.legacy.element(&tokens, false, cx))
            .child(self.other.element(&tokens, false, cx))
    }
}
#[gpui::test]
fn windowed_events_use_current_text_once_while_legacy_has_no_blur_hook(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        field: TextField::with_events(
            "",
            |owner: &mut Probe, text, _, _| owner.events.push(Seen::Submit(text)),
            FieldEvents::<Probe> {
                on_edit: Some(|owner, text, _, _| owner.events.push(Seen::Edit(text))),
                on_focus: Some(|owner, _, _| owner.events.push(Seen::Focus)),
                on_blur: Some(|owner, text, _, _| owner.events.push(Seen::Blur(text))),
            },
            window,
            cx,
        ),
        legacy: TextField::with_change(
            "",
            |_: &mut Probe, _, _, _| {},
            |owner, text, _| owner.events.push(Seen::Legacy(text)),
            window,
            cx,
        ),
        other: TextField::new("", |_: &mut Probe, _, _, _| {}, window, cx),
        events: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    probe.update_in(cx, |owner, window, cx| {
        owner.events.clear();
        owner.field.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("한글");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    probe.update_in(cx, |owner, window, cx| owner.other.focus(window, cx));
    cx.run_until_parked();
    probe.read_with(cx, |owner, _| {
        assert_eq!(
            owner.events,
            vec![
                Seen::Focus,
                Seen::Edit("한글".into()),
                Seen::Submit("한글".into()),
                Seen::Blur("한글".into())
            ]
        )
    });
    probe.update_in(cx, |owner, window, cx| {
        owner.events.clear();
        owner.legacy.focus(window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("legacy");
    cx.run_until_parked();
    probe.update_in(cx, |owner, window, cx| owner.other.focus(window, cx));
    cx.run_until_parked();
    probe.read_with(cx, |owner, _| {
        assert_eq!(owner.events, vec![Seen::Legacy("legacy".into())])
    });
}
