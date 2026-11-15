//! Real client calls over a duplex in-memory protocol peer, never a real agent.

use std::sync::Arc;
use std::time::Duration;

use chrono::DateTime;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use tokio::sync::mpsc;
use vessel_app_view::creation::{Failure, Stage, Ticket};
use vessel_app_view::machine_book::Name;
use vessel_client::NodeClient;
use vessel_proto::{
    Call, ClientFrame, ClientInfo, Delivered, ErrorCode, HarnessKind, NodeId, NodeInfo, NodeState,
    Outcome, ProtocolRange, Reply, ServerFrame, SessionId, SessionInfo, SessionMode, SessionSpec,
    Welcome, WireError,
};

use super::{Answer, Ask, FirstInput, Target, run};
use crate::Event;

const TEST_DEADLINE: Duration = Duration::from_secs(10);
const PIPE_BYTES: usize = 64 * 1024;
const EVENT_ROOM: usize = 8;
const TICKET: Ticket = Ticket::new(7);

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy)]
enum Scenario {
    Accept,
    RefusePrompt,
    DisconnectPrompt,
    RefuseCreate,
}

async fn read(stream: &mut DuplexStream) -> TestResult<Option<ClientFrame>> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = vec![0; usize::try_from(length)?];
    let _read = stream.read_exact(&mut bytes).await?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}

async fn write(stream: &mut DuplexStream, frame: ServerFrame) -> TestResult {
    let bytes = serde_json::to_vec(&frame)?;
    stream.write_u32(u32::try_from(bytes.len())?).await?;
    stream.write_all(&bytes).await?;
    Ok(())
}

async fn peer(mut stream: DuplexStream, scenario: Scenario) -> TestResult {
    assert!(matches!(
        read(&mut stream).await?,
        Some(ClientFrame::Hello(_))
    ));
    let node = NodeInfo {
        id: NodeId::random(),
        name: "fake".to_owned(),
        hostname: "fake".to_owned(),
        os: "test".to_owned(),
        arch: "test".to_owned(),
        version: "0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    };
    write(
        &mut stream,
        ServerFrame::Welcome(Welcome {
            protocol: ProtocolRange::CURRENT.max,
            node,
        }),
    )
    .await?;
    let Some(ClientFrame::Request {
        id,
        call: Call::CreateSession(request),
    }) = read(&mut stream).await?
    else {
        return Err(std::io::Error::other("create must be the first side effect").into());
    };
    if matches!(scenario, Scenario::RefuseCreate) {
        write(
            &mut stream,
            ServerFrame::Response {
                id,
                outcome: Outcome::Err(WireError::new(ErrorCode::Forbidden, "private diagnostics")),
            },
        )
        .await?;
        return Ok(());
    }
    let session = SessionInfo::new(SessionId::random(), request.spec, DateTime::UNIX_EPOCH);
    let session_id = session.id;
    write(
        &mut stream,
        ServerFrame::Response {
            id,
            outcome: Outcome::Ok(Reply::CreateSession(session)),
        },
    )
    .await?;
    if let Some(frame) = read(&mut stream).await? {
        let ClientFrame::Request {
            id,
            call: Call::PromptSession(request),
        } = frame
        else {
            return Err(std::io::Error::other("registration must not be repeated").into());
        };
        assert_eq!(request.session, session_id);
        match scenario {
            Scenario::Accept => {
                write(
                    &mut stream,
                    ServerFrame::Response {
                        id,
                        outcome: Outcome::Ok(Reply::PromptSession(Delivered::Started)),
                    },
                )
                .await?;
            }
            Scenario::RefusePrompt => {
                write(
                    &mut stream,
                    ServerFrame::Response {
                        id,
                        outcome: Outcome::Err(WireError::new(
                            ErrorCode::Conflict,
                            "private diagnostics",
                        )),
                    },
                )
                .await?;
            }
            Scenario::DisconnectPrompt => {}
            Scenario::RefuseCreate => {
                return Err(std::io::Error::other("creation already refused").into());
            }
        }
    }
    Ok(())
}

async fn execute(scenario: Scenario, input: FirstInput) -> TestResult<Vec<Answer>> {
    let (client_pipe, peer_pipe) = tokio::io::duplex(PIPE_BYTES);
    let peer = tokio::spawn(peer(peer_pipe, scenario));
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
        machine: Name::parse("fake")
            .map_err(|problem| std::io::Error::other(format!("fixture name: {problem:?}")))?,
        target: Target::New(spec),
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
        execute(Scenario::RefusePrompt, FirstInput::Text(Arc::from("hello"))),
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
        execute(Scenario::RefuseCreate, FirstInput::Deferred),
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
