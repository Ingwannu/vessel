//! Synthetic replacements only. Assertions never print the input canary.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{ClipboardItem, Entity, IntoElement, ParentElement as _, Render, TestAppContext, div};
use gpui_base::input::{Copy, Cut, InputState, Paste};
const CANARY: &str = "synthetic-secure-entry-canary";
const SENTINEL: &str = "clipboard-sentinel";

// Equality assertions remain exact without ever formatting the replacement.
#[derive(PartialEq, Eq)]
struct Hidden<T>(T);
impl<T> std::fmt::Debug for Hidden<T> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("Hidden(<redacted>)")
    }
}
fn assert_secret(actual: &str, expected: &str) {
    assert_eq!(Hidden(actual), Hidden(expected));
}

fn line(field: &SecretField) -> Option<&Entity<InputState>> {
    match &field.field.kind {
        super::super::Kind::Line(state) => Some(state),
        super::super::Kind::Lines(_, _) => None,
    }
}
fn content(presence: SecretPresence) -> SecretFieldContent {
    SecretFieldContent {
        label: "API key".into(),
        empty: "Enter a replacement".into(),
        set: "Set".into(),
        presence,
    }
}
#[test]
fn secret_carriers_redact_and_empty_never_means_delete() {
    let edit = SecretEdit::from_text(CANARY.to_owned());
    assert!(!format!("{edit:?}").contains(CANARY));
    let SecretEdit::Replace(value) = edit else {
        panic!("replacement")
    };
    assert!(!format!("{value:?}").contains(CANARY));
    assert_secret(&value.into_value(), CANARY);
    assert!(matches!(
        SecretEdit::from_text(String::new()),
        SecretEdit::Unchanged
    ));
}
#[test]
fn the_engine_accessibility_policy_is_secure_and_never_exposes_value() {
    let policy = accessibility("API key".into());
    assert_eq!(policy.role(), Some(gpui::Role::PasswordInput));
    assert_eq!(policy.label().map(AsRef::as_ref), Some("API key"));
    assert!(!policy.value_exposed());
    assert!(policy.placeholder().is_none());
}
struct Probe {
    field: SecretField,
    scheme: Scheme,
    narrow: bool,
    changes: Vec<(FieldId, SecretEdit)>,
    submitted: Option<(FieldId, SecretEdit)>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().child(
            self.field
                .element(&Tokens::of(self.scheme), self.narrow, cx),
        )
    }
}
impl Probe {
    fn new(
        presence: SecretPresence,
        scheme: Scheme,
        narrow: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        Self {
            field: SecretField::new(
                content(presence),
                |owner: &mut Self, id, edit, _, _| owner.submitted = Some((id, edit)),
                |owner: &mut Self, id, edit, _| owner.changes.push((id, edit)),
                window,
                cx,
            ),
            scheme,
            narrow,
            changes: Vec::new(),
            submitted: None,
        }
    }
}
#[gpui::test]
fn actual_masked_copy_cut_paste_and_readonly_use_the_same_engine(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for scheme in [Scheme::Light, Scheme::Dark] {
        let (view, cx) = cx.add_window_view(move |window, cx| {
            Probe::new(SecretPresence::Set, scheme, false, window, cx)
        });
        cx.update(|window, _| window.activate_window());
        view.update_in(cx, |probe, window, cx| probe.field.focus(window, cx));
        cx.run_until_parked();
        cx.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(CANARY.to_owned()));
            window.dispatch_action(Box::new(Paste), cx);
        });
        cx.run_until_parked();
        view.update_in(cx, |probe, window, cx| {
            assert_secret(&probe.field.field.text(cx), CANARY);
            let state = line(&probe.field).unwrap().read(cx);
            assert!(state.context_menu_capabilities().is_masked());
            probe.field.field.select_all(window, cx);
        });
        // GPUI dispatch is the existing open Action boundary, not a new engine.
        let copy: Box<dyn gpui::Action> = Box::new(Copy);
        for action in [copy, Box::new(Cut)] {
            cx.update(|window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(SENTINEL.to_owned()));
                window.dispatch_action(action, cx);
            });
            cx.run_until_parked();
            cx.update(|_, cx| {
                assert_eq!(
                    Hidden(
                        cx.read_from_clipboard()
                            .and_then(|item| item.text())
                            .as_deref()
                    ),
                    Hidden(Some(SENTINEL)),
                );
            });
            view.update_in(cx, |probe, _, cx| {
                assert_secret(&probe.field.field.text(cx), CANARY);
            });
        }
        view.update_in(cx, |probe, _, cx| {
            probe.field.set_editability(Editability::ReadOnly, cx);
        });
        cx.run_until_parked();
        cx.simulate_input("discarded");
        cx.update(|window, cx| window.dispatch_action(Box::new(Paste), cx));
        cx.run_until_parked();
        view.update_in(cx, |probe, _, cx| {
            assert_secret(&probe.field.field.text(cx), CANARY);
            assert!(!format!("{:?}", probe.field).contains(CANARY));
            assert!(
                probe
                    .changes
                    .iter()
                    .all(|(id, edit)| *id == probe.field.id()
                        && !format!("{edit:?}").contains(CANARY))
            );
        });
    }
}
#[gpui::test]
fn blank_set_and_erased_replacement_never_load_or_delete_existing_credentials(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for scheme in [Scheme::Light, Scheme::Dark] {
        for narrow in [false, true] {
            for presence in [SecretPresence::Absent, SecretPresence::Set] {
                let (view, cx) = cx.add_window_view(move |window, cx| {
                    Probe::new(presence, scheme, narrow, window, cx)
                });
                cx.update(|window, _| window.activate_window());
                view.update_in(cx, |probe, window, cx| {
                    assert!(probe.field.field.text(cx).is_empty());
                    let expected = match presence {
                        SecretPresence::Absent => "Enter a replacement",
                        SecretPresence::Set => "Set",
                    };
                    assert_eq!(
                        line(&probe.field)
                            .unwrap()
                            .read(cx)
                            .presentation()
                            .placeholder()
                            .as_ref(),
                        expected
                    );
                    assert_eq!(probe.field.clone().id(), probe.field.id());
                    probe.field.focus(window, cx);
                });
                cx.run_until_parked();
                cx.simulate_input(CANARY);
                cx.simulate_keystrokes("enter");
                cx.run_until_parked();
                view.update_in(cx, |probe, window, cx| {
                    let (id, edit) = probe.submitted.take().unwrap();
                    assert_eq!(id, probe.field.id());
                    let SecretEdit::Replace(value) = edit else {
                        panic!("replacement")
                    };
                    assert_secret(&value.into_value(), CANARY);
                    probe.field.field.select_all(window, cx);
                });
                cx.simulate_keystrokes("backspace");
                cx.run_until_parked();
                view.update_in(cx, |probe, _, cx| {
                    assert!(probe.field.field.text(cx).is_empty());
                    assert!(matches!(
                        probe.changes.last(),
                        Some((_, SecretEdit::Unchanged))
                    ));
                });
            }
        }
    }
}

mod accessibility;
