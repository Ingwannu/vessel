use super::*;
use std::collections::BTreeMap;
use vessel_proto::conversation::{AnswerValue, FieldKind, QuestionField};
use vessel_proto::{PermissionOption, PermissionOptionKind, ToolCallId};

#[test]
fn cancellation_another_option_or_another_request_is_not_an_allowed_success() {
    let selected = PermissionOptionId::new("provider-approval");
    let request = PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("tool"),
        options: vec![PermissionOption {
            id: selected.clone(),
            name: "Proceed".into(),
            kind: PermissionOptionKind::AllowOnce,
        }],
    };
    let chosen = PermissionOutcome::Selected {
        option: selected.clone(),
    };
    assert_eq!(permission(&request, &selected, request.id, &chosen), Ok(()));
    assert_eq!(
        permission(&request, &selected, PermissionId::random(), &chosen),
        Err(Unconfirmed)
    );
    assert_eq!(
        permission(
            &request,
            &selected,
            request.id,
            &PermissionOutcome::Cancelled
        ),
        Err(Unconfirmed)
    );
    let different = PermissionOptionId::new("different");
    assert_eq!(
        permission(
            &request,
            &selected,
            request.id,
            &PermissionOutcome::Selected {
                option: different.clone()
            }
        ),
        Err(Unconfirmed)
    );
    assert_eq!(
        permission(
            &request,
            &different,
            request.id,
            &PermissionOutcome::Selected {
                option: different.clone()
            }
        ),
        Err(Unconfirmed)
    );
}

#[test]
fn question_confirmation_matches_exact_values_and_does_not_turn_cancel_into_decline() {
    let name = "answer".parse().unwrap();
    let request = Question {
        id: QuestionId::random(),
        message: "Choose".into(),
        tool_call: None,
        fields: vec![QuestionField {
            name,
            title: None,
            description: None,
            required: true,
            kind: FieldKind::Boolean { default: None },
        }],
    };
    let values = BTreeMap::from([("answer".parse().unwrap(), AnswerValue::Boolean(true))]);
    let accepted = QuestionAnswer::Accept {
        values: values.clone(),
    };
    let outcome = QuestionOutcome::Answered { values };
    assert_eq!(question(&request, &accepted, request.id, &outcome), Ok(()));
    assert_eq!(
        question(&request, &accepted, QuestionId::random(), &outcome),
        Err(Unconfirmed)
    );
    for other in [
        QuestionOutcome::Declined,
        QuestionOutcome::Cancelled,
        QuestionOutcome::Answered {
            values: BTreeMap::new(),
        },
    ] {
        assert_eq!(
            question(&request, &accepted, request.id, &other),
            Err(Unconfirmed)
        );
    }
    assert_eq!(
        question(
            &request,
            &QuestionAnswer::Decline,
            request.id,
            &QuestionOutcome::Declined
        ),
        Ok(())
    );
    assert_eq!(
        question(
            &request,
            &QuestionAnswer::Decline,
            request.id,
            &QuestionOutcome::Cancelled
        ),
        Err(Unconfirmed)
    );
    let invalid = QuestionAnswer::Accept {
        values: BTreeMap::new(),
    };
    assert_eq!(
        question(
            &request,
            &invalid,
            request.id,
            &QuestionOutcome::Answered {
                values: BTreeMap::new()
            }
        ),
        Err(Unconfirmed)
    );
}
