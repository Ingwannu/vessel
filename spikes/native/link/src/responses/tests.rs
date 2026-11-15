//! Actor debug output carries request correlation, never a person's answer.
use super::*;
use std::collections::BTreeMap;
use vessel_app_view::{
    connection::ConnectionEpochs,
    devices::MachineKey,
    responses::{Registry, Target},
};
use vessel_proto::{NodeId, SessionId, conversation::AnswerValue};

#[test]
fn answer_payloads_stay_private_through_the_enclosing_command_debug() {
    let canary = "synthetic-private-question-answer";
    let connection = ConnectionEpochs::default()
        .admit(MachineKey::from_book("scratch"), NodeId::random())
        .unwrap();
    for response in [
        Response::Permission {
            id: PermissionId::random(),
            option: PermissionOptionId::new(canary),
        },
        Response::Question {
            id: QuestionId::random(),
            answer: QuestionAnswer::Accept {
                values: BTreeMap::from([(
                    "answer".parse().unwrap(),
                    AnswerValue::Text(canary.into()),
                )]),
            },
        },
    ] {
        let request = response.request();
        let pending = Registry::default()
            .begin(
                Target {
                    connection: connection.clone(),
                    session: SessionId::random(),
                },
                request,
            )
            .unwrap();
        let command = crate::Command::Respond(Ask::new(pending, response).unwrap());
        let debug = format!("{command:?}");
        assert!(!debug.contains(canary));
        assert!(debug.contains(&format!("{request:?}")));
    }
}
