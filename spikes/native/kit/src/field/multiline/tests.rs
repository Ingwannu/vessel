//! Source identity rides real multiline engine events through the same listener.
use super::*;
use crate::{
    Scheme, SchemeChoice, Tokens,
    field::{Editability, FieldId},
};
use gpui::{IntoElement, ParentElement as _, Render, TestAppContext, div};
struct Probe {
    field: TextField,
    changed: Option<(FieldId, String)>,
    submitted: Vec<(FieldId, String)>,
}
impl Probe {
    fn new(minimum: FormMinimum, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        Self {
            field: TextField::form_multiline_with_source(
                "Prompt",
                |this: &mut Self, id, text, _, _| this.submitted.push((id, text)),
                |this: &mut Self, id, text, cx| {
                    this.changed = Some((id, text));
                    cx.notify();
                },
                minimum,
                window,
                cx,
            ),
            changed: None,
            submitted: vec![],
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(self.field.element(&Tokens::of(Scheme::Dark), false, cx))
    }
}
#[gpui::test]
fn identified_multiline_plain_enter_and_mod_submit_report_the_actual_engine_source(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for minimum in [FormMinimum::Default, FormMinimum::Expanded] {
        let (view, cx) = cx.add_window_view(move |window, cx| Probe::new(minimum, window, cx));
        cx.update(|window, _| window.activate_window());
        view.update_in(cx, |this, window, cx| this.field.focus(window, cx));
        cx.run_until_parked();
        cx.simulate_input("first");
        cx.simulate_keystrokes("enter");
        cx.simulate_input("둘째");
        cx.run_until_parked();
        view.read_with(cx, |this, cx| {
            assert_eq!(this.field.text(cx), "first\n둘째");
            assert_eq!(this.changed, Some((this.field.id(), "first\n둘째".into())));
            assert!(this.submitted.is_empty());
            assert_eq!(this.field.clone().id(), this.field.id());
            assert_eq!(this.field.form_minimum, minimum);
        });
        let submit = cx.read(|cx| {
            if cx.platform_info().is_mac() {
                "cmd-enter"
            } else {
                "ctrl-enter"
            }
        });
        cx.simulate_keystrokes(submit);
        cx.run_until_parked();
        view.read_with(cx, |this, _| {
            assert_eq!(this.submitted, [(this.field.id(), "first\n둘째".into())]);
        });
    }
}
#[gpui::test]
fn the_same_identified_engine_obeys_existing_readonly_and_reenable(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|window, cx| Probe::new(FormMinimum::Expanded, window, cx));
    view.update_in(cx, |this, window, cx| {
        this.field.set_text("kept", window, cx);
        this.field.focus(window, cx);
        this.field.set_editability(Editability::ReadOnly, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("blocked");
    cx.run_until_parked();
    view.read_with(cx, |this, cx| {
        assert_eq!(this.field.text(cx), "kept");
        assert_eq!(this.changed, None);
    });
    view.update_in(cx, |this, window, cx| {
        this.field.set_editability(Editability::Editable, cx);
        this.field.focus(window, cx);
    });
    cx.simulate_input("allowed");
    cx.run_until_parked();
    view.read_with(cx, |this, cx| {
        assert_ne!(this.field.text(cx), "kept");
        assert_eq!(
            this.changed.as_ref().map(|(source, _)| *source),
            Some(this.field.id())
        );
        assert!(this.submitted.is_empty());
    });
}
