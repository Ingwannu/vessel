use super::*;

#[test]
fn old_request_cannot_transfer_to_a_new_state_with_changed_graph_at_the_same_address() {
    let mut changed_method = field("text");
    let ControlContent::TextField { submit, .. } = &mut changed_method.content else {
        panic!("field");
    };
    *submit = "act".parse().expect("changed method");
    let disabled = control(
        "disabled-wrapper",
        ControlContent::Button {
            button: Button {
                title: "Disabled".parse().expect("title"),
                availability: Availability::Disabled,
                behavior: ButtonBehavior::Popover {
                    surface: Surface {
                        controls: vec![field("text")],
                    },
                },
            },
        },
    );
    for changed_controls in [vec![], vec![changed_method], vec![disabled]] {
        let (installation, address) = entry(vec![field("text")]);
        let mut registry = Registry::default();
        publish(&mut registry, installation);
        let mut original = SurfaceState::new(&registry, address.clone()).expect("original state");
        original
            .edit(&id("text"), text("captured literal"))
            .expect("edit");
        let request = original
            .submit(&registry, &id("text"))
            .expect("original request");
        // Removing the previous entry also removes the inherited per-entry
        // generation comparison. Simulate a node reusing that exact address.
        let empty = registry
            .begin(address.installation.node, HostSupport::Supported)
            .expect("refresh");
        assert_eq!(
            registry
                .apply(empty, catalog(empty.node(), vec![]))
                .expect("authoritative withdrawal"),
            SnapshotApplied::Applied
        );
        let changed = ClientDocument::new(
            vec![
                "submit".parse().expect("method"),
                "act".parse().expect("method"),
            ],
            vec![Contribution {
                id: address.contribution.clone(),
                title: "Home".parse().expect("title"),
                content: ContributionContent::Screen {
                    surface: Surface {
                        controls: changed_controls,
                    },
                },
            }],
            vec![],
        )
        .expect("changed valid graph");
        publish(
            &mut registry,
            Installation::from_document(address.installation.clone(), changed),
        );
        let receiving = SurfaceState::new(&registry, address).expect("new state from graph B");
        receiving
            .check_current(&registry)
            .expect("receiving graph is current");
        assert_eq!(
            receiving.revalidate(&registry, &request),
            Err(SurfaceProblem::Catalog(Problem::Stale))
        );
    }
}

#[test]
fn unchanged_graph_reconstruction_keeps_the_captured_literal_after_later_edits() {
    let (installation, address) = entry(vec![field("text")]);
    let mut registry = Registry::default();
    publish(&mut registry, installation.clone());
    let mut original = SurfaceState::new(&registry, address.clone()).expect("original state");
    original
        .edit(&id("text"), text("submitted literal"))
        .expect("edit");
    let request = original.submit(&registry, &id("text")).expect("request");
    original
        .edit(&id("text"), text("later draft"))
        .expect("later edit");
    // An equivalent graph at a distinct allocation is not a semantic change.
    let mut rebuilt = Registry::default();
    publish(
        &mut rebuilt,
        Installation::from_document(
            address.installation.clone(),
            installation.document().clone(),
        ),
    );
    let receiving = SurfaceState::new(&rebuilt, address).expect("equivalent receiving state");
    receiving
        .revalidate(&rebuilt, &request)
        .expect("unchanged graph");
    assert!(
        matches!(request.submission(), Submission::Field { value: FieldValue::Text(text), .. } if text.as_str() == "submitted literal")
    );
    assert_eq!(original.value(&id("text")), Some(&text("later draft")));
}
