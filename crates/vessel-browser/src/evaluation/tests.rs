//! Foreign-result validation is independent from whether page code is trustworthy.
use super::*;
use crate::cdp::requests::Evaluated;

fn observed(value: serde_json::Value) -> Evaluated {
    serde_json::from_value(serde_json::json!({"result":{"value":value}})).unwrap()
}
#[test]
fn valid_result_remains_opaque_and_keeps_its_truncation_fact() {
    let secret = r#"{"private":"own signed-in fixture"}"#;
    let result = observed(serde_json::json!({"kind":"value","text":secret,"completeness":"whole"}));
    let content = finish(&result).unwrap();
    assert_eq!(content.text.as_str(), secret);
    assert_eq!(content.completeness, TextCompleteness::Whole);
    assert!(!format!("{content:?}").contains("signed-in"));
    let preview = observed(
        serde_json::json!({"kind":"value","text":"\"partial\"","completeness":"truncated"}),
    );
    assert_eq!(
        finish(&preview).unwrap().completeness,
        TextCompleteness::Truncated
    );
}
#[test]
fn malformed_unknown_and_oversized_results_never_become_successful_echoes() {
    for value in [
        serde_json::json!({"kind":"value","text":"{broken","completeness":"whole"}),
        serde_json::json!({"kind":"value","text":"null"}),
        serde_json::json!({"kind":"value","text":"null","completeness":"whole","other":1}),
        serde_json::json!({"kind":"unknown"}),
        serde_json::json!({"kind":"value","text":"x".repeat(MAX_PAGE_TEXT_BYTES + 1),"completeness":"whole"}),
    ] {
        assert!(finish(&observed(value)).is_err());
    }
    for kind in ["exception", "not_function"] {
        let error = finish(&observed(serde_json::json!({"kind":kind}))).unwrap_err();
        assert!(!format!("{error:?}").contains("private"));
    }
}
#[test]
fn wrapper_overflow_refuses_before_execution_without_returning_the_function() {
    let source =
        RequiredInput::try_from("\"".repeat(vessel_proto::browser::MAX_BROWSER_INPUT_BYTES))
            .unwrap();
    assert!(matches!(
        expression(&source),
        Err(EvaluationError::Input(_))
    ));
    let invalid: Evaluated = serde_json::from_str(
        r#"{"result":{},"exceptionDetails":{"text":"private page exception"}}"#,
    )
    .unwrap();
    let error = finish(&invalid).unwrap_err();
    assert!(!format!("{error:?}").contains("private page exception"));
}
