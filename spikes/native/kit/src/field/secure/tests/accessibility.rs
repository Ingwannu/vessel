//! Observe the real emitted secure node beside an ordinary value-bearing input.
//! Assertions retain no snapshot and never format a replacement on failure.
use super::super::super::{
    Kind,
    accessibility_tests::{self, Role},
};
use super::*;
use gpui_base::input::InputAccessibility;

const REFERENCE: &str = "ordinary-nonsecret-reference";
struct AccessiblePair {
    secure: SecretField,
    ordinary: TextField,
    scheme: Scheme,
    narrow: bool,
}
impl Render for AccessiblePair {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        div()
            .child(self.secure.element(&tokens, self.narrow, cx))
            .child(self.ordinary.element(&tokens, self.narrow, cx))
    }
}
impl AccessiblePair {
    fn new(scheme: Scheme, narrow: bool, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let secure = SecretField::new(
            content(SecretPresence::Set),
            |_, _, _, _, _| {},
            |_, _, _, _| {},
            window,
            cx,
        );
        let ordinary = TextField::new("", |_, _, _, _| {}, window, cx);
        let Kind::Line(state) = &ordinary.kind else {
            panic!("ordinary line")
        };
        state.update(cx, |state, _| {
            state.set_accessibility(
                InputAccessibility::new(Some(gpui::Role::TextInput))
                    .with_label(Some("ordinary reference".into())),
            );
        });
        ordinary.set_text(REFERENCE, window, cx);
        secure.focus(window, cx);
        Self {
            secure,
            ordinary,
            scheme,
            narrow,
        }
    }
}
#[gpui::test]
fn actual_secure_node_has_no_value_while_ordinary_reference_does(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for scheme in [Scheme::Light, Scheme::Dark] {
        for narrow in [false, true] {
            let (view, cx) = cx
                .add_window_view(move |window, cx| AccessiblePair::new(scheme, narrow, window, cx));
            cx.run_until_parked();
            let handle = cx.update(|window, _| window.window_handle());
            assert!(cx.simulate_accessibility_activation(handle));
            cx.run_until_parked();
            cx.simulate_input(CANARY);
            cx.run_until_parked();
            view.update_in(cx, |pair, _, cx| {
                assert_secret(&pair.secure.field.text(cx), CANARY);
                cx.notify();
            });
            cx.run_until_parked();
            cx.update(|window, _| {
                assert!(window.is_a11y_active());
                let raw = window.debug_a11y_tree_json().unwrap();
                assert!(!raw.contains(CANARY));
                assert!(raw.contains(REFERENCE));
                let snapshot = accessibility_tests::read(window).unwrap().unwrap();
                let secure: Vec<_> = snapshot.nodes(Role::PasswordInput).collect();
                assert_eq!(secure.len(), 1);
                assert_eq!(secure[0].label.as_deref(), Some("API key"));
                assert!(secure[0].value.is_none());
                let ordinary: Vec<_> = snapshot.nodes(Role::TextInput).collect();
                assert_eq!(ordinary.len(), 1);
                assert_eq!(ordinary[0].value.as_deref(), Some(REFERENCE));
            });
        }
    }
}
