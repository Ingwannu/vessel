//! Native discovery filters disabled/invalid/duplicate skills and preserves invocation.

use super::*;

fn catalog() -> Catalog {
    Catalog::reported(serde_json::from_str(r#"{"data":[{"skills":[
      {"name":"review","description":"Review changes","path":"/skills/review/SKILL.md","enabled":true},
      {"name":"disabled","description":"hidden","path":"/disabled","enabled":false},
      {"name":"invalid name","description":"invalid","path":"/invalid","enabled":true},
      {"name":"empty","description":"invalid path","path":" ","enabled":true},
      {"name":"review","description":"duplicate","path":"/other","enabled":true}
    ]}]}"#).unwrap())
}

#[test]
fn only_enabled_unique_valid_skills_are_advertised_and_native_invocation_uses_their_path() {
    let catalog = catalog();
    let commands = catalog.commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name.as_str(), "review");
    assert_eq!(commands[0].description, "Review changes");
    assert_eq!(commands[0].kind, Some(CommandKind::Skill));
    assert_eq!(
        catalog
            .input(&[ContentBlock::Text {
                text: "  /review   changes \n ".to_owned()
            }])
            .unwrap(),
        vec![
            UserInput::Skill {
                name: "review".to_owned(),
                path: "/skills/review/SKILL.md".to_owned()
            },
            UserInput::Text {
                text: "$review changes".to_owned()
            },
        ]
    );
}

#[test]
fn inline_unknown_and_attached_prompts_retain_their_original_content() {
    let catalog = catalog();
    for text in [
        "use /review here",
        "/unknown x",
        "/review/other",
        "/disabled",
        "/",
    ] {
        let content = [ContentBlock::Text {
            text: text.to_owned(),
        }];
        assert_eq!(
            catalog.input(&content).unwrap(),
            mapper::user_input(&content).unwrap()
        );
    }
    let attached = [
        ContentBlock::Text {
            text: "/review".to_owned(),
        },
        ContentBlock::ResourceLink {
            uri: "file:///a.rs".to_owned(),
            name: "a.rs".to_owned(),
        },
    ];
    assert_eq!(
        catalog.input(&attached).unwrap(),
        mapper::user_input(&attached).unwrap()
    );
}

#[test]
fn provider_report_bounds_preserve_utf8_and_discovery_limit() {
    let response = SkillsListResponse {
        data: vec![SkillsListEntry {
            skills: (0..AgentCommand::MAX_COMMANDS.saturating_add(1))
                .map(|index| SkillMetadata {
                    name: format!("skill-{index}"),
                    description: "한".repeat(AgentCommand::MAX_DESCRIPTION),
                    path: format!("/skills/{index}"),
                    enabled: true,
                })
                .collect(),
        }],
    };
    let commands = Catalog::reported(response).commands();
    assert_eq!(commands.len(), AgentCommand::MAX_COMMANDS);
    assert!(
        commands
            .iter()
            .all(|command| command.description.len() <= AgentCommand::MAX_DESCRIPTION)
    );
    assert!(commands[0].description.ends_with('한'));
}
