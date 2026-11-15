use super::*;
use crate::plugins::{HostSupport, Installation, SnapshotApplied};
use vessel_proto::plugins::{
    Availability, Button, ButtonBehavior, ChoiceOption, ClientDocument, Contribution,
    ContributionKind, Control, MenuEntry, PluginAddress, RuntimeGeneration,
};

#[test]
fn all_fields_share_one_edit_budget_and_refusal_preserves_the_previous_draft() {
    const CHUNK_BYTES: usize = 16 * 1024;
    const FILLED_FIELDS: usize = MAX_SURFACE_EDIT_BYTES.div_euclid(CHUNK_BYTES);
    let (installation, address) = entry(
        (0..=FILLED_FIELDS)
            .map(|index| field(&format!("field-{index}")))
            .collect(),
    );
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    let mut state = SurfaceState::new(&registry, address).expect("surface");
    let value = text(&"x".repeat(CHUNK_BYTES));
    for index in 0..FILLED_FIELDS {
        state
            .edit(&id(&format!("field-{index}")), value.clone())
            .expect("within shared cap");
    }
    let last = id(&format!("field-{FILLED_FIELDS}"));
    assert_eq!(
        state.edit(&last, value.clone()),
        Err(SurfaceProblem::Capacity)
    );
    assert_eq!(state.value(&last), Some(&text("")));
    state
        .edit(&id("field-0"), text(""))
        .expect("release old edit bytes");
    state
        .edit(&last, value.clone())
        .expect("released space reused");
    assert_eq!(state.value(&last), Some(&value));
}
use vessel_proto::{NodeId, RawJson};

fn id(raw: &str) -> ContributionId {
    raw.parse().expect("control identity")
}
fn text(raw: &str) -> FieldValue {
    FieldValue::Text(raw.parse().expect("bounded text"))
}
fn control(name: &str, content: ControlContent) -> Control {
    Control {
        id: id(name),
        content,
    }
}
fn action() -> Action {
    Action::Invoke {
        method: "act".parse().expect("method"),
        input: RawJson::from_text(r#"{"literal":true}"#).expect("data"),
    }
}
fn field(name: &str) -> Control {
    control(
        name,
        ControlContent::TextField {
            label: "Text".parse().expect("label"),
            submit: "submit".parse().expect("method"),
        },
    )
}
fn entry(controls: Vec<Control>) -> (Installation, ContributionAddress) {
    let installation = PluginAddress {
        node: NodeId::random(),
        plugin: "sample".parse().expect("plugin"),
        generation: RuntimeGeneration::random(),
    };
    let document = ClientDocument::new(
        vec![
            "submit".parse().expect("method"),
            "act".parse().expect("method"),
        ],
        vec![Contribution {
            id: id("home"),
            title: "Home".parse().expect("caption"),
            content: ContributionContent::Screen {
                surface: Surface { controls },
            },
        }],
        vec![],
    )
    .expect("validated graph");
    let address = ContributionAddress {
        installation: installation.clone(),
        kind: ContributionKind::Screen,
        contribution: id("home"),
    };
    (Installation::from_document(installation, document), address)
}
fn publish(registry: &mut Registry, installation: Installation) {
    let ticket = registry
        .begin(installation.address().node, HostSupport::Supported)
        .expect("begin");
    assert_eq!(
        registry.apply(ticket, Ok(vec![installation])),
        Ok(SnapshotApplied::Applied)
    );
}

#[test]
fn form_values_are_typed_retained_and_captured_without_interpreting_input() {
    let (installation, address) = entry(vec![
        field("text"),
        control(
            "choice",
            ControlContent::Choice {
                label: "Choice".parse().expect("label"),
                options: vec![ChoiceOption {
                    id: id("one"),
                    title: "One".parse().expect("title"),
                }],
                selected: None,
                submit: "submit".parse().expect("method"),
            },
        ),
        control(
            "toggle",
            ControlContent::Toggle {
                label: "Toggle".parse().expect("label"),
                state: ToggleState::Off,
                submit: "submit".parse().expect("method"),
            },
        ),
    ]);
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    let mut state = SurfaceState::new(&registry, address.clone()).expect("surface");
    assert_eq!(state.address(), &address);
    assert_eq!(state.surface().controls.len(), 3);
    assert_eq!(state.value(&id("text")), Some(&text("")));
    state
        .edit(&id("text"), text("/do-not-execute PRIVATE-SURFACE-CANARY"))
        .expect("edit");
    let request = state
        .submit(&registry, &id("text"))
        .expect("captured submit");
    state.edit(&id("text"), text("later edit")).expect("later");
    assert_eq!(request.address(), &address);
    let Submission::Field {
        control,
        method,
        value,
    } = request.submission()
    else {
        panic!("field");
    };
    assert_eq!(control, &id("text"));
    assert_eq!(method.as_str(), "submit");
    assert_eq!(value, &text("/do-not-execute PRIVATE-SURFACE-CANARY"));
    for diagnostic in [
        format!("{request:?}"),
        format!("{:?}", request.submission()),
        format!("{value:?}"),
        format!("{state:?}"),
    ] {
        assert!(!diagnostic.contains("PRIVATE-SURFACE-CANARY"));
    }
    assert_eq!(
        state.edit(&id("choice"), FieldValue::Choice(Some(id("missing")))),
        Err(SurfaceProblem::Value)
    );
    assert_eq!(
        state.edit(&id("text"), FieldValue::Toggle(ToggleState::On)),
        Err(SurfaceProblem::Value)
    );
    state
        .edit(&id("choice"), FieldValue::Choice(Some(id("one"))))
        .expect("declared choice");
    state
        .edit(&id("toggle"), FieldValue::Toggle(ToggleState::On))
        .expect("toggle");
    assert!(
        matches!(state.submit(&registry, &id("choice")).expect("choice").submission(), Submission::Field { value: FieldValue::Choice(Some(selected)), .. } if selected == &id("one"))
    );
    assert!(matches!(
        state
            .submit(&registry, &id("toggle"))
            .expect("toggle")
            .submission(),
        Submission::Field {
            value: FieldValue::Toggle(ToggleState::On),
            ..
        }
    ));
}

#[test]
fn offline_or_replaced_surface_refuses_queued_intentions_without_host_fallback() {
    let (installation, address) = entry(vec![field("text")]);
    let mut registry = Registry::default();
    publish(&mut registry, installation.clone());
    let mut state = SurfaceState::new(&registry, address.clone()).expect("surface");
    state
        .edit(&id("text"), text("retained draft"))
        .expect("edit");
    let request = state.submit(&registry, &id("text")).expect("request");
    state
        .revalidate(&registry, &request)
        .expect("still current");
    let mut elsewhere = installation.address().clone();
    elsewhere.node = NodeId::random();
    publish(
        &mut registry,
        Installation::from_document(elsewhere, installation.document().clone()),
    );
    registry.offline(address.installation.node);
    state
        .edit(&id("text"), text("offline edit"))
        .expect("retain offline edit");
    assert_eq!(
        state.submit(&registry, &id("text")),
        Err(SurfaceProblem::Catalog(Problem::Offline))
    );
    assert_eq!(
        state.revalidate(&registry, &request),
        Err(SurfaceProblem::Catalog(Problem::Offline))
    );
    let mut replacement = address.installation.clone();
    replacement.generation = RuntimeGeneration::random();
    publish(
        &mut registry,
        Installation::from_document(replacement, installation.document().clone()),
    );
    assert_eq!(
        state.revalidate(&registry, &request),
        Err(SurfaceProblem::Catalog(Problem::Stale))
    );
    assert_eq!(state.value(&id("text")), Some(&text("offline edit")));
}

fn menu_item(name: &str, availability: Availability, behavior: ButtonBehavior) -> MenuEntry {
    MenuEntry::Item {
        id: id(name),
        title: name.parse().expect("caption"),
        availability,
        behavior,
    }
}
#[test]
fn menu_paths_and_disabled_ancestors_cannot_be_bypassed_by_nested_controls() {
    let button = |availability, behavior| ControlContent::Button {
        button: Button {
            title: "Action".parse().expect("caption"),
            availability,
            behavior,
        },
    };
    let (installation, address) = entry(vec![
        control(
            "menu",
            button(
                Availability::Enabled,
                ButtonBehavior::Menu {
                    items: vec![
                        MenuEntry::Separator {
                            id: id("separator"),
                        },
                        menu_item(
                            "enabled",
                            Availability::Enabled,
                            ButtonBehavior::Action { action: action() },
                        ),
                        menu_item(
                            "hidden",
                            Availability::Hidden,
                            ButtonBehavior::Action { action: action() },
                        ),
                        menu_item(
                            "disabled",
                            Availability::Disabled,
                            ButtonBehavior::Popover {
                                surface: Surface {
                                    controls: vec![field("nested")],
                                },
                            },
                        ),
                    ],
                },
            ),
        ),
        control(
            "hidden-button",
            button(
                Availability::Hidden,
                ButtonBehavior::Action { action: action() },
            ),
        ),
    ]);
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    let mut state = SurfaceState::new(&registry, address).expect("surface");
    assert!(
        matches!(state.activate(&registry, &id("menu"), &[id("enabled")]).expect("action").submission(), Submission::Action(selected) if selected == &action())
    );
    assert_eq!(
        state.activate(&registry, &id("menu"), &[id("hidden")]),
        Err(SurfaceProblem::Disabled)
    );
    assert_eq!(
        state.activate(&registry, &id("hidden-button"), &[]),
        Err(SurfaceProblem::Disabled)
    );
    assert_eq!(
        state.activate(&registry, &id("menu"), &[id("separator")]),
        Err(SurfaceProblem::Value)
    );
    assert_eq!(
        state.activate(&registry, &id("menu"), &[id("missing")]),
        Err(SurfaceProblem::Missing)
    );
    assert_eq!(
        state.edit(&id("nested"), text("bypass")),
        Err(SurfaceProblem::Disabled)
    );
    assert_eq!(
        state.submit(&registry, &id("nested")),
        Err(SurfaceProblem::Disabled)
    );
}

#[test]
fn another_surface_or_registry_cannot_reuse_a_captured_request() {
    let (installation, address) = entry(vec![field("text")]);
    let mut registry = Registry::default();
    publish(&mut registry, installation);
    let state = SurfaceState::new(&registry, address).expect("surface");
    let request = state.submit(&registry, &id("text")).expect("request");
    let (other, address) = entry(vec![field("text")]);
    publish(&mut registry, other);
    let other = SurfaceState::new(&registry, address).expect("other surface");
    assert_eq!(
        other.revalidate(&registry, &request),
        Err(SurfaceProblem::Catalog(Problem::Stale))
    );
    let mut replaced_registry = Registry::default();
    publish(
        &mut replaced_registry,
        Installation::from_document(
            state.address().installation.clone(),
            ClientDocument::new(
                vec!["submit".parse().expect("method")],
                vec![Contribution {
                    id: id("home"),
                    title: "Changed".parse().expect("title"),
                    content: ContributionContent::Screen {
                        surface: Surface { controls: vec![] },
                    },
                }],
                vec![],
            )
            .expect("changed graph"),
        ),
    );
    assert_eq!(
        state.revalidate(&replaced_registry, &request),
        Err(SurfaceProblem::Catalog(Problem::Stale))
    );
}
