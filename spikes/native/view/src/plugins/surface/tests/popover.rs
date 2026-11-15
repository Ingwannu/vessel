use super::*;

#[test]
fn popup_resolution_reuses_live_graph_and_every_menu_ancestor() {
    for availability in [
        Availability::Enabled,
        Availability::Disabled,
        Availability::Hidden,
    ] {
        let (installation, address) = entry(vec![control(
            "trigger",
            ControlContent::Button {
                button: Button {
                    title: "Root".parse().expect("caption"),
                    availability: Availability::Enabled,
                    behavior: ButtonBehavior::Menu {
                        items: vec![MenuEntry::Item {
                            id: id("form"),
                            title: "Form".parse().expect("caption"),
                            availability,
                            behavior: ButtonBehavior::Popover {
                                surface: Surface {
                                    controls: vec![field("inside")],
                                },
                            },
                        }],
                    },
                },
            },
        )]);
        let mut registry = Registry::default();
        publish(&mut registry, installation);
        let state = SurfaceState::new(&registry, address.clone()).expect("state");
        let result = state.popover(&registry, &id("trigger"), &[id("form")]);
        if availability == Availability::Enabled {
            let (title, body) = result.expect("enabled declared popup");
            assert_eq!(title.as_str(), "Form");
            assert_eq!(body.controls[0].id, id("inside"));
            assert_eq!(
                state.popover(&registry, &id("trigger"), &[]),
                Err(SurfaceProblem::Value)
            );
            assert_eq!(
                state.popover(&registry, &id("trigger"), &[id("missing")]),
                Err(SurfaceProblem::Missing)
            );
        } else {
            assert_eq!(result, Err(SurfaceProblem::Disabled));
        }
        registry.offline(address.installation.node);
        assert_eq!(
            state.popover(&registry, &id("trigger"), &[id("form")]),
            Err(Problem::Offline.into())
        );
    }
}
