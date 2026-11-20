use std::collections::BTreeMap;

use serde_json::{Value, json};
use vessel_proto::conversation::AnswerValue;
use vessel_proto::{PermissionOptionId, PermissionOutcome, QuestionAnswer};

use super::*;

fn request(tool: &str, input: Value, suggestions: Vec<Value>) -> CanUseTool {
    CanUseTool {
        tool_name: tool.to_owned(),
        input,
        tool_use_id: "toolu_1".to_owned(),
        permission_suggestions: suggestions,
        title: None,
    }
}

fn permission(opened: &Opened) -> &PermissionRequest {
    let Shown::Permission(request) = &opened.shown else {
        panic!("{opened:?}");
    };
    request
}

fn names(opened: &Opened) -> Vec<String> {
    permission(opened)
        .options
        .iter()
        .map(|option| option.name.clone())
        .collect()
}

#[test]
fn a_suggestion_is_offered_as_its_words_and_only_when_an_allow_can_carry_it() {
    let suggestions = vec![
        json!({"type": "addRules", "rules": [{"toolName": "Read"}, {"toolName": "Bash",
               "ruleContent": "npm test"}], "behavior": "allow", "destination": "projectSettings"}),
        json!({"type": "replaceRules", "rules": [{"toolName": "Edit"}], "behavior": "allow",
               "destination": "userSettings"}),
        json!({"type": "setMode", "mode": "acceptEdits", "destination": "session"}),
        json!({"type": "addDirectories", "directories": ["/srv/a", "/srv/b"],
               "destination": "localSettings"}),
        // Not offered: a rule that denies, a removal, a switch into bypass,
        // a place this build does not know, a shape it does not know.
        json!({"type": "addRules", "rules": [{"toolName": "Bash"}], "behavior": "deny",
               "destination": "session"}),
        json!({"type": "removeRules", "rules": [{"toolName": "Bash"}], "behavior": "allow",
               "destination": "session"}),
        json!({"type": "setMode", "mode": "bypassPermissions", "destination": "session"}),
        json!({"type": "setMode", "mode": "plan", "destination": "elsewhere"}),
        json!({"type": "grantEverything"}),
    ];
    let mut asks = Asks::default();
    let opened = asks.tool("c1".to_owned(), &request("Bash", json!({}), suggestions));
    assert_eq!(
        names(&opened),
        [
            "Allow",
            "Always allow Read, Bash(npm test) (this project)",
            "Always allow Edit (all your projects)",
            "Switch to acceptEdits (this session)",
            "Allow access to /srv/a, /srv/b (this project, for you)",
            "Reject",
        ]
    );
    // Each option's id names the suggestion it carries.
    let ids: Vec<_> = permission(&opened)
        .options
        .iter()
        .map(|option| option.id.as_str().to_owned())
        .collect();
    assert_eq!(
        ids,
        [
            "allow",
            "suggestion-0",
            "suggestion-1",
            "suggestion-2",
            "suggestion-3",
            "reject"
        ]
    );
}

#[test]
fn an_answer_is_the_chosen_permission_once_and_a_cancel_is_a_withdrawal() {
    let mut asks = Asks::default();
    let input = json!({"command": "ls"});
    let opened = asks.tool("c1".to_owned(), &request("Bash", input.clone(), Vec::new()));
    let id = permission(&opened).id;
    let settled = asks
        .answer(
            id,
            &PermissionOutcome::Selected {
                option: PermissionOptionId::new("allow"),
            },
        )
        .unwrap();
    assert_eq!(settled.request_id, "c1");
    assert_eq!(
        settled.permission,
        Permission::Allow {
            updated_input: input.clone(),
            updated_permissions: Vec::new(),
            tool_use_id: "toolu_1".to_owned(),
        }
    );
    assert!(matches!(
        asks.answer(id, &PermissionOutcome::Cancelled),
        Err(InputError::PermissionSettled(_))
    ));

    let opened = asks.tool("c2".to_owned(), &request("Bash", input, Vec::new()));
    let id = permission(&opened).id;
    let settled = asks.answer(id, &PermissionOutcome::Cancelled).unwrap();
    assert!(matches!(
        settled.permission,
        Permission::Deny { ref message, .. } if message == choices::WITHDRAWN
    ));
}

#[test]
fn the_oldest_request_gives_way_and_is_still_answered() {
    let mut asks = Asks::default();
    let first = asks.tool("c0".to_owned(), &request("Bash", json!({}), Vec::new()));
    let oldest = permission(&first).id;
    for index in 1..MAX_WAITING {
        let opened = asks.tool(format!("c{index}"), &request("Bash", json!({}), Vec::new()));
        assert!(opened.displaced.is_none());
    }
    let over = asks.tool("over".to_owned(), &request("Bash", json!({}), Vec::new()));
    let Some((asked, settled)) = over.displaced else {
        panic!("the oldest gives way");
    };
    assert_eq!(asked, Asked::Permission(oldest));
    assert_eq!(settled.request_id, "c0");
    assert!(matches!(settled.permission, Permission::Deny { .. }));
    assert!(matches!(
        asks.answer(oldest, &PermissionOutcome::Cancelled),
        Err(InputError::PermissionSettled(_))
    ));
    assert_eq!(asks.withdraw_all().len(), MAX_WAITING);
    assert!(asks.withdraw_all().is_empty());
}

#[test]
fn what_the_agent_withdraws_is_settled_and_remembered() {
    let mut asks = Asks::default();
    let opened = asks.plan(
        "c1".to_owned(),
        &request("ExitPlanMode", json!({}), Vec::new()),
    );
    let id = permission(&opened).id;
    assert_eq!(asks.withdrawn("other"), None);
    assert_eq!(asks.withdrawn("c1"), Some(Asked::Permission(id)));
    assert!(matches!(
        asks.answer(id, &PermissionOutcome::Cancelled),
        Err(InputError::PermissionSettled(_))
    ));
}

#[test]
fn a_question_without_options_takes_text_and_an_unfit_one_is_not_shown() {
    let mut asks = Asks::default();
    let free = json!({"questions": [{"question": "Why?", "header": "Reason"}]});
    let opened = asks
        .question(
            "c1".to_owned(),
            &request("AskUserQuestion", free, Vec::new()),
        )
        .unwrap();
    let Shown::Question(question) = &opened.shown else {
        panic!("{opened:?}");
    };
    let values = BTreeMap::from([(
        "question_1".parse().unwrap(),
        AnswerValue::Text("Because.".to_owned()),
    )]);
    let (_, settled) = asks
        .reply(question.id, QuestionAnswer::Accept { values })
        .unwrap();
    let Permission::Allow { updated_input, .. } = settled.permission else {
        panic!("{settled:?}");
    };
    assert_eq!(updated_input["answers"], json!({"Why?": "Because."}));

    for unfit in [
        json!({"questions": []}),
        json!({"questions": "Why?"}),
        json!({"questions": vec![json!({"question": "?"}); Question::MAX_FIELDS + 1]}),
    ] {
        assert!(
            asks.question(
                "c2".to_owned(),
                &request("AskUserQuestion", unfit, Vec::new())
            )
            .is_err()
        );
    }
}
