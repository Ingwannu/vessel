//! Real client calls over a duplex in-memory protocol peer, never a real agent.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use vessel_app_view::creation::{Failure, Stage, Ticket};
use vessel_app_view::machine_book::Name;
use vessel_client::NodeClient;
use vessel_proto::{ClientInfo, ErrorCode, HarnessKind, SessionId, SessionMode, SessionSpec};

use super::{Answer, Ask, FirstInput, Target, run};
use crate::Event;

const TEST_DEADLINE: Duration = Duration::from_secs(10);
const PIPE_BYTES: usize = 64 * 1024;
const EVENT_ROOM: usize = 8;
const TICKET: Ticket = Ticket::new(7);

mod peer;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy)]
enum Scenario {
    Accept,
    RefusePrompt(ErrorCode),
    DisconnectPrompt,
    RefuseCreate(ErrorCode),
    Retry(SessionId),
    WrongNode,
}

async fn execute(scenario: Scenario, input: FirstInput) -> TestResult<Vec<Answer>> {
    let (client_pipe, peer_pipe) = tokio::io::duplex(PIPE_BYTES);
    let peer = tokio::spawn(peer::serve(peer_pipe, scenario));
    let (reader, writer) = tokio::io::split(client_pipe);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, vessel_rpc::MAX_FRAME),
        ClientInfo {
            name: "test".to_owned(),
            version: "0".to_owned(),
        },
        &vessel_rpc::ClientConfig::default(),
    )
    .await?;
    let spec = SessionSpec::new(
        HarnessKind::Codex,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let ask = Ask {
        ticket: TICKET,
        message: vessel_proto::timeline::ClientMessageId::random(),
        expected_node: if matches!(scenario, Scenario::WrongNode) {
            vessel_proto::NodeId::random()
        } else {
            client.node().id
        },
        machine: Name::parse("fake")
            .map_err(|problem| std::io::Error::other(format!("fixture name: {problem:?}")))?,
        target: match scenario {
            Scenario::Retry(session) => Target::Registered(session),
            Scenario::Accept
            | Scenario::RefusePrompt(_)
            | Scenario::RefuseCreate(_)
            | Scenario::DisconnectPrompt
            | Scenario::WrongNode => Target::New(spec),
        },
        input,
    };
    let (tx, mut rx) = mpsc::channel(EVENT_ROOM);
    run::create(&client, ask, &tx).await;
    drop(tx);
    drop(client);
    peer.await??;
    let mut answers = Vec::new();
    while let Some(event) = rx.recv().await {
        let Event::Creation(answer) = event else {
            return Err(std::io::Error::other("creation events only").into());
        };
        answers.push(answer);
    }
    Ok(answers)
}

#[tokio::test]
async fn registration_and_success_are_correlated_and_in_order() {
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(Scenario::Accept, FirstInput::Text(Arc::from("hello"))),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        answers.as_slice(),
        [
            Answer::Registered { ticket: TICKET, .. },
            Answer::Progress {
                ticket: TICKET,
                stage: Stage::InitialInput
            },
            Answer::Complete(TICKET)
        ]
    ));
}

#[tokio::test]
async fn a_prompt_refusal_retains_the_registered_session_and_does_not_retry_create() {
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(
            Scenario::RefusePrompt(ErrorCode::Conflict),
            FirstInput::Text(Arc::from("hello")),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        answers.as_slice(),
        [
            Answer::Registered { ticket: TICKET, .. },
            Answer::Progress { .. },
            Answer::Failed {
                ticket: TICKET,
                stage: Stage::InitialInput,
                failure: Failure::Refused(ErrorCode::Conflict)
            }
        ]
    ));
    assert!(!format!("{answers:?}").contains("private diagnostics"));
}

#[tokio::test]
async fn a_disconnected_prompt_is_uncertain_and_a_create_refusal_has_no_resource() {
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(
            Scenario::DisconnectPrompt,
            FirstInput::Text(Arc::from("hello")),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        answers.last(),
        Some(Answer::Failed {
            stage: Stage::InitialInput,
            failure: Failure::OutcomeUnknown,
            ..
        })
    ));
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(
            Scenario::RefuseCreate(ErrorCode::Forbidden),
            FirstInput::Deferred,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        answers.as_slice(),
        [Answer::Failed {
            stage: Stage::Resource,
            failure: Failure::Refused(ErrorCode::Forbidden),
            ..
        }]
    ));
}

#[tokio::test]
async fn deferred_payload_is_not_sent_by_the_creation_owner() {
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(Scenario::Accept, FirstInput::Deferred),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        answers.as_slice(),
        [Answer::Registered { .. }, Answer::Complete(TICKET)]
    ));
}

#[tokio::test]
async fn refused_first_input_retries_only_the_registered_session() {
    let session = SessionId::random();
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(
            Scenario::Retry(session),
            FirstInput::Text(Arc::from("retry")),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        matches!(answers.first(), Some(Answer::Registered { session: info, .. }) if info.id == session)
    );
    assert!(matches!(answers.last(), Some(Answer::Complete(TICKET))));
}

#[tokio::test]
async fn a_reused_book_name_cannot_retarget_a_captured_submission() {
    let answers = tokio::time::timeout(
        TEST_DEADLINE,
        execute(Scenario::WrongNode, FirstInput::Deferred),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        answers.as_slice(),
        [Answer::Failed {
            failure: Failure::Refused(ErrorCode::Conflict),
            ..
        }]
    ));
}

#[tokio::test]
async fn internal_errors_cannot_prove_that_create_or_first_input_had_no_effect() {
    for (scenario, stage) in [
        (Scenario::RefuseCreate(ErrorCode::Internal), Stage::Resource),
        (
            Scenario::RefusePrompt(ErrorCode::Internal),
            Stage::InitialInput,
        ),
    ] {
        let answers = tokio::time::timeout(
            TEST_DEADLINE,
            execute(scenario, FirstInput::Text(Arc::from("first input"))),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(matches!(answers.last(), Some(Answer::Failed {
            stage: failed, failure: Failure::OutcomeUnknown, ..
        }) if *failed == stage));
        assert!(
            !answers
                .iter()
                .any(|answer| matches!(answer, Answer::Complete(_)))
        );
        assert!(!format!("{answers:?}").contains("private diagnostics"));
    }
}
