//! Registration and first delivery have separate failure boundaries.

use super::{Answer, Ask, FirstInput, Target};
use crate::connections::{RequestFailure, SelectedConnection};
use std::time::Duration;
use tokio::sync::mpsc;
use vessel_app_view::creation::{Failure, Stage};
use vessel_client::CallError;
use vessel_proto::conversation::PromptSession;
use vessel_proto::{
    ContentBlock, CreateSession, Delivery, GetSession, SendInput, SessionId, SessionInfo,
    SessionInput,
};

/// A legacy RPC bound is distinct from a provisioning-operation TTL.
const CALL_DEADLINE: Duration = Duration::from_secs(120);

pub(crate) async fn create(
    client: &SelectedConnection,
    ask: Ask,
    events: &mpsc::Sender<crate::Event>,
) {
    if ask.context.node() != ask.expected_node
        || ask.context.machine().as_str() != ask.machine.as_str()
    {
        let _closed = events
            .send(crate::Event::Creation(
                ask.refused(Failure::Refused(vessel_proto::ErrorCode::Conflict)),
            ))
            .await;
        return;
    }
    let source = ask.source;
    let ticket = ask.ticket;
    let stage = match &ask.target {
        Target::New(_) => Stage::Resource,
        Target::Registered(_) => Stage::InitialInput,
    };
    let info = match register(client, ask.target).await {
        Ok(info) => info,
        Err(failure) => {
            let _closed = events
                .send(crate::Event::Creation(Answer::Failed {
                    source,
                    ticket,
                    stage,
                    failure,
                }))
                .await;
            return;
        }
    };
    let session = info.id;
    if events
        .send(crate::Event::Creation(Answer::Registered {
            source,
            ticket,
            machine: ask.machine,
            node: ask.context.node(),
            context: ask.context.clone(),
            session: Box::new(info),
        }))
        .await
        .is_err()
    {
        return;
    }
    let FirstInput::Text(text) = ask.input else {
        let _closed = events
            .send(crate::Event::Creation(Answer::Complete(source, ticket)))
            .await;
        return;
    };
    let _closed = events
        .send(crate::Event::Creation(Answer::Progress {
            source,
            ticket,
            stage: Stage::InitialInput,
        }))
        .await;
    let answer = match deliver(client, session, &text).await {
        Ok(()) => Answer::Complete(source, ticket),
        Err(failure) => Answer::Failed {
            source,
            ticket,
            stage: Stage::InitialInput,
            failure,
        },
    };
    let _closed = events.send(crate::Event::Creation(answer)).await;
}

async fn register(client: &SelectedConnection, target: Target) -> Result<SessionInfo, Failure> {
    let expected = match &target {
        Target::New(_) => None,
        Target::Registered(session) => Some(*session),
    };
    let register = async {
        match target {
            Target::New(spec) => client.call(CreateSession { spec }).await,
            Target::Registered(session) => client.call(GetSession { session }).await,
        }
    };
    let info = tokio::time::timeout(CALL_DEADLINE, register)
        .await
        .map_err(|_elapsed| Failure::OutcomeUnknown)?
        .map_err(|error| failure(&error))?;
    if expected.is_some_and(|expected| info.id != expected) {
        return Err(Failure::Refused(vessel_proto::ErrorCode::Conflict));
    }
    Ok(info)
}

async fn deliver(
    client: &SelectedConnection,
    session: SessionId,
    text: &str,
) -> Result<(), Failure> {
    let send = async {
        let prompted = client
            .call(PromptSession {
                session,
                content: vec![ContentBlock::Text {
                    text: text.to_owned(),
                }],
                delivery: Delivery::Start,
                notify: false,
            })
            .await;
        match prompted {
            Ok(_delivered) => Ok(()),
            // The typed client refused this method before transport. Only this
            // known preflight may fall back to the pre-protocol-10 send shape.
            Err(RequestFailure::Call(CallError::NotInProtocol { .. })) => client
                .call(SendInput {
                    session,
                    input: SessionInput::Prompt {
                        content: vec![ContentBlock::Text {
                            text: text.to_owned(),
                        }],
                    },
                })
                .await
                .map(|_ack| ()),
            Err(error) => Err(error),
        }
    };
    tokio::time::timeout(CALL_DEADLINE, send)
        .await
        .map_err(|_elapsed| Failure::OutcomeUnknown)?
        .map_err(|error| failure(&error))
}

/// Transport loss may hide a committed side effect; never turn it into retry.
fn failure(error: &RequestFailure) -> Failure {
    match error {
        RequestFailure::Refused(_) => Failure::OutcomeUnknown,
        RequestFailure::Call(error) => call_failure(error),
    }
}

fn call_failure(error: &CallError) -> Failure {
    match error {
        CallError::Remote(error) => match error.code {
            // Session persistence can fail after the process starts. A generic
            // internal error is not evidence that its side effects never ran.
            // Shutdown can also return Unavailable after the child spawned.
            vessel_proto::ErrorCode::Internal | vessel_proto::ErrorCode::Unavailable => {
                Failure::OutcomeUnknown
            }
            vessel_proto::ErrorCode::InvalidRequest
            | vessel_proto::ErrorCode::Unauthorized
            | vessel_proto::ErrorCode::Forbidden
            | vessel_proto::ErrorCode::ProtocolMismatch
            | vessel_proto::ErrorCode::NotFound
            | vessel_proto::ErrorCode::Conflict
            | vessel_proto::ErrorCode::Unsupported
            | vessel_proto::ErrorCode::HarnessNotInstalled
            | vessel_proto::ErrorCode::SpawnFailed => Failure::Refused(error.code),
        },
        CallError::NotInProtocol { .. }
        | CallError::UnassignedProtocol { .. }
        | CallError::RequestIdsExhausted => Failure::Unsupported,
        CallError::Disconnected | CallError::UnexpectedReply(_) => Failure::OutcomeUnknown,
    }
}
