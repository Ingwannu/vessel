//! Decode only GPUI's SDK debug snapshot, never domain/wire JSON. Opaque SDK
//! map keys are not control identities and never leave this decoding edge.
//! No raw snapshot or node value implements Debug or is printed on failure.
use super::*;
use gpui::{IntoElement, ParentElement as _, Render, TestAppContext};
use gpui_base::input::InputAccessibility;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    TextInput,
    PasswordInput,
    #[serde(other)]
    Other,
}
#[derive(Deserialize)]
pub(super) struct Aria {
    role: Role,
    pub(super) label: Option<String>,
    pub(super) value: Option<String>,
}
#[derive(Deserialize)]
struct Node {
    aria: Aria,
}
#[derive(Deserialize)]
pub(super) struct Snapshot {
    nodes: BTreeMap<String, Node>,
}
impl Snapshot {
    pub(super) fn nodes(&self, role: Role) -> impl Iterator<Item = &Aria> {
        self.nodes
            .values()
            .map(|node| &node.aria)
            .filter(move |aria| aria.role == role)
    }
}
pub(super) fn read(window: &gpui::Window) -> Option<Result<Snapshot, serde_json::Error>> {
    window
        .debug_a11y_tree_json()
        .map(|snapshot| serde_json::from_str(&snapshot))
}
const REFERENCE_VALUE: &str = "nonsecret-reference-value";
struct Probe {
    field: TextField,
}
impl Render for Probe {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        gpui::div().child(
            self.field
                .element(&crate::Tokens::of(crate::Scheme::Dark), false, cx),
        )
    }
}
#[gpui::test]
fn default_inactive_then_real_adapter_activation_observes_the_existing_editor(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|window, cx| {
        let field = TextField::new("", |_, _, _, _| {}, window, cx);
        let Kind::Line(state) = &field.kind else {
            panic!("line factory")
        };
        state.update(cx, |state, _| {
            state.set_accessibility(
                InputAccessibility::new(Some(gpui::Role::TextInput))
                    .with_label(Some("ordinary reference".into())),
            )
        });
        field.set_text(REFERENCE_VALUE, window, cx);
        Probe { field }
    });
    cx.run_until_parked();
    let handle = cx.update(|window, _| {
        assert!(!window.is_a11y_active());
        assert!(read(window).is_none());
        window.window_handle()
    });
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    cx.update(|window, _| {
        assert!(window.is_a11y_active());
        let snapshot = read(window).unwrap().unwrap();
        let nodes: Vec<_> = snapshot.nodes(Role::TextInput).collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label.as_deref(), Some("ordinary reference"));
        assert_eq!(nodes[0].value.as_deref(), Some(REFERENCE_VALUE));
        assert!(snapshot.nodes(Role::PasswordInput).next().is_none());
    });
}
