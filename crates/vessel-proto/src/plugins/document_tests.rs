//! Whole-document rejection tests exercise the render/dispatch trust boundary.

use super::*;

#[test]
fn declaration_parser_errors_seal_every_input_bearing_diagnostic_level() {
    use std::error::Error;

    for text in [
        r#"{"methods":[],"contributions":[],"themes":[],"PRIVATE-GRAPH-CANARY":true}"#,
        r#"{"methods":[],"contributions":[{"id":"a","title":"A","content":{"kind":"PRIVATE-GRAPH-CANARY"}}],"themes":[]}"#,
        r#"{"methods":[],"contributions":"PRIVATE-GRAPH-CANARY","themes":[]}"#,
    ] {
        let error = ClientDocument::decode(text.as_bytes()).expect_err("invalid graph");
        let mut cause: &dyn Error = &error;
        loop {
            assert!(!format!("{cause:?} {cause}").contains("PRIVATE-GRAPH-CANARY"));
            match cause.source() {
                Some(next) => cause = next,
                None => break,
            }
        }
        let error = serde_json::from_str::<ClientDocument>(text).expect_err("serde graph");
        let mut cause: &dyn Error = &error;
        loop {
            assert!(!format!("{cause:?} {cause}").contains("PRIVATE-GRAPH-CANARY"));
            match cause.source() {
                Some(next) => cause = next,
                None => break,
            }
        }
    }
}
use crate::plugins::PanelLocation;
use crate::plugins::declarations::{ChoiceOption, CommandContext, Control, PanelContext};

fn screen(id: &str, controls: Vec<Control>) -> Contribution {
    Contribution {
        id: id.parse().expect("id"),
        title: "Screen".parse().expect("caption"),
        content: ContributionContent::Screen {
            surface: Surface { controls },
        },
    }
}

fn text(id: &str) -> Control {
    Control {
        id: id.parse().expect("id"),
        content: ControlContent::Text {
            text: "Hello\n안녕하세요".parse().expect("content"),
        },
    }
}

#[test]
fn unknown_controls_and_executable_fields_refuse_at_decode() {
    let raw = r#"{"methods":[],"themes":[],"contributions":[{"id":"home","title":"Home","content":{"kind":"screen","surface":{"controls":[{"id":"a","content":{"kind":"javascript","script":"secret()"}}]}}}]}"#;
    assert!(ClientDocument::decode(raw.as_bytes()).is_err());
    let valid = ClientDocument::new(vec![], vec![screen("home", vec![text("text")])], vec![])
        .expect("graph");
    let encoded = serde_json::to_string(&valid).expect("encode");
    let extra = encoded.replace(
        "\"kind\":\"text\"",
        "\"kind\":\"text\",\"fontFamily\":\"evil\"",
    );
    assert!(ClientDocument::decode(extra.as_bytes()).is_err());
    assert_eq!(
        ClientDocument::decode(encoded.as_bytes()).expect("decode"),
        valid
    );
}

#[test]
fn duplicate_identity_and_dangling_screen_or_method_refuse_entire_graph() {
    assert!(matches!(
        ClientDocument::new(
            vec![],
            vec![screen("same", vec![]), screen("same", vec![])],
            vec![]
        ),
        Err(DocumentError::Duplicate)
    ));
    assert!(matches!(
        ClientDocument::new(
            vec![],
            vec![screen("home", vec![text("same"), text("same")])],
            vec![]
        ),
        Err(DocumentError::Duplicate)
    ));
    let action = Contribution {
        id: "open".parse().expect("id"),
        title: "Open".parse().expect("caption"),
        content: ContributionContent::Command {
            keywords: vec![],
            context: CommandContext::Global,
            action: Action::OpenScreen {
                screen: "missing".parse().expect("id"),
            },
        },
    };
    assert!(matches!(
        ClientDocument::new(vec![], vec![action], vec![]),
        Err(DocumentError::MissingScreen)
    ));
    let control = Control {
        id: "field".parse().expect("id"),
        content: ControlContent::TextField {
            label: "Field".parse().expect("caption"),
            submit: "submit".parse().expect("method"),
        },
    };
    assert!(matches!(
        ClientDocument::new(vec![], vec![screen("home", vec![control])], vec![]),
        Err(DocumentError::MissingMethod)
    ));
}

#[test]
fn method_contribution_byte_and_control_budgets_are_whole_result_refusals() {
    let excess = vec![b' '; MAX_DOCUMENT_BYTES.saturating_add(1)];
    assert!(matches!(
        ClientDocument::decode(&excess),
        Err(DocumentError::Bytes)
    ));
    let methods = (0..MAX_METHODS.saturating_add(1))
        .map(|n| format!("method{n}").parse().expect("method"))
        .collect();
    assert!(matches!(
        ClientDocument::new(methods, vec![], vec![]),
        Err(DocumentError::Methods)
    ));
    let entries = (0..MAX_CONTRIBUTIONS.saturating_add(1))
        .map(|n| screen(&format!("screen-{n}"), vec![]))
        .collect();
    assert!(matches!(
        ClientDocument::new(vec![], entries, vec![]),
        Err(DocumentError::Contributions)
    ));
    let controls = (0..MAX_CONTROLS.saturating_add(1))
        .map(|n| text(&format!("text-{n}")))
        .collect();
    assert!(matches!(
        ClientDocument::new(vec![], vec![screen("home", controls)], vec![]),
        Err(DocumentError::Controls)
    ));
}

#[test]
fn deep_popovers_and_container_trees_share_the_same_depth_budget() {
    let mut surface = Surface { controls: vec![] };
    for depth in 0..MAX_DEPTH.saturating_add(1) {
        surface = Surface {
            controls: vec![Control {
                id: format!("level-{depth}").parse().expect("id"),
                content: ControlContent::Stack { surface },
            }],
        };
    }
    let contribution = screen("home", surface.controls);
    assert!(matches!(
        ClientDocument::new(vec![], vec![contribution], vec![]),
        Err(DocumentError::Depth)
    ));
}

#[test]
fn choices_require_distinct_options_and_a_present_selected_value() {
    let option = ChoiceOption {
        id: "a".parse().expect("id"),
        title: "A".parse().expect("caption"),
    };
    let control = Control {
        id: "choice".parse().expect("id"),
        content: ControlContent::Choice {
            label: "Choose".parse().expect("caption"),
            options: vec![option],
            selected: Some("missing".parse().expect("id")),
            submit: "submit".parse().expect("method"),
        },
    };
    assert!(matches!(
        ClientDocument::new(
            vec!["submit".parse().expect("method")],
            vec![screen("home", vec![control])],
            vec![]
        ),
        Err(DocumentError::MissingChoice)
    ));
}

#[test]
fn panel_locations_are_not_invented_or_duplicated() {
    for locations in [
        vec![],
        vec![PanelLocation::Workspace, PanelLocation::Workspace],
    ] {
        let contribution = Contribution {
            id: "panel".parse().expect("id"),
            title: "Panel".parse().expect("caption"),
            content: ContributionContent::Panel {
                context: PanelContext::Workspace,
                locations,
                surface: Surface { controls: vec![] },
            },
        };
        assert!(ClientDocument::new(vec![], vec![contribution], vec![]).is_err());
    }
}

#[test]
fn typed_construction_cannot_bypass_the_encoded_byte_budget() {
    let content: crate::plugins::ContentText = "x"
        .repeat(MAX_DOCUMENT_BYTES.div_euclid(MAX_CONTRIBUTIONS))
        .parse()
        .expect("bounded content");
    let controls = (0..MAX_CONTROLS)
        .map(|number| Control {
            id: format!("text-{number}").parse().expect("id"),
            content: ControlContent::Text {
                text: content.clone(),
            },
        })
        .collect();
    assert!(matches!(
        ClientDocument::new(vec![], vec![screen("home", controls)], vec![]),
        Err(DocumentError::Bytes)
    ));
}

#[test]
fn sidebar_header_and_footer_may_reuse_a_local_identity() {
    let mut items = vec![screen("home", vec![])];
    for placement in [
        super::super::SidebarPlacement::Header,
        super::super::SidebarPlacement::Footer,
    ] {
        items.push(Contribution {
            id: "item".parse().expect("id"),
            title: "Open".parse().expect("caption"),
            content: ContributionContent::Sidebar {
                placement,
                screen: "home".parse().expect("screen"),
            },
        });
    }
    assert!(ClientDocument::new(vec![], items, vec![]).is_ok());
}

#[test]
fn plugin_slash_registration_refuses_provider_only_names_and_duplicate_names() {
    let slash = |id: &str, name: &str| Contribution {
        id: id.parse().expect("id"),
        title: "Review".parse().expect("caption"),
        content: ContributionContent::SlashCommand {
            name: name.parse().expect("common command name"),
            description: "Review changes".parse().expect("caption"),
            argument_hint: None,
            context: PanelContext::Workspace,
            method: "review".parse().expect("method"),
        },
    };
    assert!(matches!(
        ClientDocument::new(
            vec!["review".parse().expect("method")],
            vec![slash("a", "Upper")],
            vec![]
        ),
        Err(DocumentError::SlashName(_))
    ));
    assert!(matches!(
        ClientDocument::new(
            vec!["review".parse().expect("method")],
            vec![slash("a", "review"), slash("b", "review")],
            vec![]
        ),
        Err(DocumentError::Duplicate)
    ));
}

#[test]
fn nested_menus_validate_references_and_keep_separator_identity() {
    use super::super::{Availability, MenuEntry};
    let item = MenuEntry::Item {
        id: "same".parse().expect("id"),
        title: "Open".parse().expect("caption"),
        availability: Availability::Enabled,
        behavior: ButtonBehavior::Action {
            action: Action::OpenScreen {
                screen: "home".parse().expect("screen"),
            },
        },
    };
    let control = Control {
        id: "menu".parse().expect("id"),
        content: ControlContent::Button {
            button: Button {
                title: "Menu".parse().expect("caption"),
                availability: Availability::Enabled,
                behavior: ButtonBehavior::Menu {
                    items: vec![
                        MenuEntry::Separator {
                            id: "same".parse().expect("id"),
                        },
                        item,
                    ],
                },
            },
        },
    };
    assert!(matches!(
        ClientDocument::new(vec![], vec![screen("home", vec![control])], vec![]),
        Err(DocumentError::Duplicate)
    ));
}
