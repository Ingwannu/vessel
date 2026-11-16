//! Foreign output tests use completed facts, not a fake browser or a wire grant.
use super::*;
use vessel_proto::browser::{
    ActionOutcome, CssPoint, ElementArgs, InputText, RequiredInput, ValueArgs, WaitArgs,
    WaitBudget, WaitFor, WaitMatch,
};

fn id() -> BrowserId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}
fn fill() -> BrowserCommand {
    BrowserCommand::Fill(ValueArgs {
        target: ElementArgs {
            browser_id: id(),
            element: "@e1".parse().unwrap(),
        },
        value: InputText::try_from("synthetic input".to_owned()).unwrap(),
    })
}
#[test]
fn same_tab_different_action_or_element_is_not_a_successful_fill() {
    for outcome in [
        ActionOutcome::Select {
            element: "@e1".parse().unwrap(),
            value: InputText::try_from("synthetic input".to_owned()).unwrap(),
        },
        ActionOutcome::Fill {
            element: "@e2".parse().unwrap(),
        },
    ] {
        let reply = render(
            &fill(),
            BrowserResult::Applied {
                browser_id: id(),
                outcome,
            },
        )
        .unwrap();
        assert_eq!(reply.is_error, Some(true));
        assert!(reply.structured_content.is_none());
    }
    let reply = render(
        &fill(),
        BrowserResult::Applied {
            browser_id: id(),
            outcome: ActionOutcome::Fill {
                element: "@e1".parse().unwrap(),
            },
        },
    )
    .unwrap();
    let result = &reply.structured_content.as_ref().unwrap()["result"];
    assert_eq!(result["command"], "fill");
    assert_eq!(result["ref"], "@e1");
    assert!(result.get("value").is_none());
    assert!(result.get("outcome").is_none());
}

#[test]
fn points_are_flat_observed_coordinates_and_absence_does_not_become_null_or_zero() {
    let position: CssPoint = serde_json::from_str(r#"{"x":12.5,"y":8}"#).unwrap();
    let result = actions::metadata(
        &id(),
        ActionOutcome::Drag {
            source: "@e1".parse().unwrap(),
            target: "@e2".parse().unwrap(),
            source_position: position,
            target_position: position,
        },
    );
    assert_eq!(result["sourceRef"], "@e1");
    assert_eq!(result["targetRef"], "@e2");
    assert_eq!(result["sourceX"], 12.5);
    assert_eq!(result["targetY"], 8.0);
    assert!(result.get("source_position").is_none());
    let result = actions::metadata(
        &id(),
        ActionOutcome::Type {
            element: None,
            position: None,
        },
    );
    for field in ["ref", "x", "y"] {
        assert!(result.get(field).is_none());
    }
}

#[test]
fn successful_wait_kind_must_match_the_admitted_condition() {
    let command = BrowserCommand::Wait(WaitArgs {
        browser_id: id(),
        condition: WaitFor::Text(RequiredInput::try_from("own needle".to_owned()).unwrap()),
        budget: WaitBudget::Default,
    });
    let wrong = render(
        &command,
        BrowserResult::Matched {
            browser_id: id(),
            matched: WaitMatch::Url,
        },
    )
    .unwrap();
    assert_eq!(wrong.is_error, Some(true));
    let reply = render(
        &command,
        BrowserResult::Matched {
            browser_id: id(),
            matched: WaitMatch::Text,
        },
    )
    .unwrap();
    let result = &reply.structured_content.as_ref().unwrap()["result"];
    assert_eq!(result["command"], "wait");
    assert_eq!(result["matched"], "text");
}
