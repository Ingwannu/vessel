//! The engine's actual input events retain the source used by both callbacks.
use super::*;
use crate::{Scheme, SchemeChoice, Tokens};
use gpui::{IntoElement, ParentElement as _, Render, TestAppContext, div};

struct Probe {
    field: TextField,
    changed: Option<(FieldId, String)>,
    submitted: Option<(FieldId, String)>,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(self.field.element(&Tokens::of(Scheme::Dark), false, cx))
    }
}
#[gpui::test]
fn keyboard_events_report_the_same_source_as_the_retained_editor(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|window, cx| Probe {
        field: TextField::with_source(
            "",
            |owner: &mut Probe, id, text, _, _| owner.submitted = Some((id, text)),
            |owner: &mut Probe, id, text, _| owner.changed = Some((id, text)),
            window,
            cx,
        ),
        changed: None,
        submitted: None,
    });
    cx.update(|window, _cx| window.activate_window());
    view.update_in(cx, |view, window, cx| view.field.focus(window, cx));
    cx.run_until_parked();
    cx.simulate_input("chosen answer");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    view.update_in(cx, |view, _, _| {
        let expected = Some((view.field.id(), "chosen answer".to_owned()));
        assert_eq!(view.changed, expected);
        assert_eq!(view.submitted, expected);
        assert_eq!(view.field.clone().id(), view.field.id());
    });
}
