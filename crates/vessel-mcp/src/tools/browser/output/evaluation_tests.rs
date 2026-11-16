//! Result JSON is observed page output, not a request echo or a second parse.
use super::*;
use vessel_proto::browser::{EvaluateArgs, MAX_PAGE_TEXT_BYTES, PageText, RequiredInput};

fn command() -> BrowserCommand {
    BrowserCommand::Evaluate(EvaluateArgs {
        browser_id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        function: RequiredInput::try_from("() => 'request source only'".to_owned()).unwrap(),
        element: None,
    })
}

#[test]
fn evaluation_preserves_observed_json_string_without_reparsing_or_request_echo() {
    let command = command();
    let observed = r#"{"observed":"한글","count":3}"#;
    let reply = render(
        &command,
        BrowserResult::Evaluation {
            browser_id: command.target().unwrap().clone(),
            json: PageText::capped(observed),
        },
    )
    .unwrap();
    let result = &reply.structured_content.as_ref().unwrap()["result"];
    assert_eq!(result["command"], "evaluate");
    assert_eq!(result["resultJson"], observed);
    assert_eq!(result["truncated"], false);
    assert!(result.get("json").is_none());
    assert!(
        !serde_json::to_string(&reply)
            .unwrap()
            .contains("request source only")
    );
}

#[test]
fn evaluation_carries_the_existing_utf8_truncation_fact_without_a_false_json_value() {
    let command = command();
    let oversized = "한".repeat(MAX_PAGE_TEXT_BYTES);
    let capped = PageText::capped(&oversized);
    let expected = capped.text.as_str().to_owned();
    let reply = render(
        &command,
        BrowserResult::Evaluation {
            browser_id: command.target().unwrap().clone(),
            json: capped,
        },
    )
    .unwrap();
    let result = &reply.structured_content.as_ref().unwrap()["result"];
    assert_eq!(result["truncated"], true);
    assert_eq!(result["resultJson"], expected);
    assert!(expected.len() <= MAX_PAGE_TEXT_BYTES);
    assert!(oversized.starts_with(&expected));
    assert!(serde_json::from_str::<Value>(&expected).is_err());
}
