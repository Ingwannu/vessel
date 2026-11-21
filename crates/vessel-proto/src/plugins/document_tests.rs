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
use crate::plugins::declarations::{
    Action, Button, ButtonBehavior, ChoiceOption, CommandContext, Control, ControlContent,
    PanelContext, Surface,
};

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

/// A settings screen whose card binds two documents, and a command that
/// opens it (ADR 0115, "Surfaces").
const BOUND: &str = r#"{"methods":[],"themes":[],"contributions":[
  {"id":"display","title":"Display","content":{"kind":"settings_screen","surface":{"controls":[
    {"id":"card","content":{"kind":"card","surface":{"controls":[
      {"id":"on","content":{"kind":"settings_switch","settings":"display","key":"showMetadata","label":"Show metadata","hint":"In the list"}},
      {"id":"color","content":{"kind":"settings_select","settings":"display","key":"color","label":"Color","options":[{"id":"blue","title":"Blue"}]}},
      {"id":"name","content":{"kind":"settings_input","settings":"names","key":"name","label":"Name","placeholder":"Anyone","mode":"secret"}}
    ]}}}
  ]}}},
  {"id":"open","title":"Display settings","content":{"kind":"command","keywords":[],"context":"global","action":{"kind":"open_settings","screen":"display"}}}
]}"#;

fn declared(text: &str) -> SettingsDeclarations {
    serde_json::from_str(text).expect("declarations")
}

#[test]
fn settings_controls_bind_documents_and_need_the_surfaces_protocol() {
    let document = ClientDocument::decode(BOUND.as_bytes()).expect("bound");
    let bound: Vec<_> = document.bound_settings().map(SettingsId::as_str).collect();
    assert_eq!(bound, ["display", "names"]);
    assert_eq!(
        Some(document.required_client()),
        crate::plugins::PLUGIN_SURFACES_PROTOCOL
    );
    let plain = ClientDocument::new(vec![], vec![screen("home", vec![text("text")])], vec![])
        .expect("plain");
    assert_eq!(
        Some(plain.required_client()),
        crate::plugins::PLUGIN_CATALOG_PROTOCOL
    );
    assert_eq!(plain.bound_settings().count(), 0);
    // What the check found is not sent, and is found again on decode.
    let sent = serde_json::to_string(&document).expect("encode");
    assert!(!sent.contains("bindings") && !sent.contains("required_client"));
    assert_eq!(
        serde_json::from_str::<ClientDocument>(&sent).expect("again"),
        document
    );

    let both = declared(r#"[{"id":"display","version":2},{"id":"names","version":1}]"#);
    assert!(document.check_settings(&both).is_ok());
    let one = declared(r#"[{"id":"display","version":2}]"#);
    assert!(matches!(
        document.check_settings(&one),
        Err(DocumentError::UndeclaredSettings)
    ));
    assert!(matches!(
        document.check_settings(&SettingsDeclarations::default()),
        Err(DocumentError::UndeclaredSettings)
    ));
    assert!(
        plain
            .check_settings(&SettingsDeclarations::default())
            .is_ok()
    );
}

#[test]
fn open_settings_needs_a_settings_screen_and_a_select_needs_distinct_options() {
    let refused = |text: &str| ClientDocument::decode(text.as_bytes()).expect_err("refused");
    let plain_screen = BOUND.replace(r#""kind":"settings_screen""#, r#""kind":"screen""#);
    assert!(matches!(
        refused(&plain_screen),
        DocumentError::MissingScreen
    ));
    let options = r#"[{"id":"blue","title":"Blue"}]"#;
    assert!(matches!(
        refused(&BOUND.replace(options, "[]")),
        DocumentError::EmptyOptions
    ));
    let twice = r#"[{"id":"blue","title":"Blue"},{"id":"blue","title":"Also blue"}]"#;
    assert!(matches!(
        refused(&BOUND.replace(options, twice)),
        DocumentError::Duplicate
    ));
    assert!(matches!(
        refused(&BOUND.replace("showMetadata", "show.metadata")),
        DocumentError::Decode(_)
    ));
}

#[test]
fn settings_keys_are_ascii_identifiers() {
    use crate::plugins::SettingsKey;

    for good in ["showMetadata", "_private", "a1", "A"] {
        assert!(good.parse::<SettingsKey>().is_ok(), "{good}");
    }
    let long = "a".repeat(65);
    for bad in ["", "1a", "a-b", "a.b", "a b", "é", long.as_str()] {
        assert!(bad.parse::<SettingsKey>().is_err(), "{bad}");
    }
}
