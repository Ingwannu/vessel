//! Registration and first delivery have separate failure boundaries.

use super::{Answer, Ask, FirstInput, Target};
use std::time::Duration;
use tokio::sync::mpsc;
use vessel_app_view::creation::{Failure, Stage};
use vessel_client::{CallError, NodeClient};
use vessel_proto::{ContentBlock, Delivery, ProtocolVersion, SessionId, SessionInfo, SessionInput};

/// A legacy RPC bound is distinct from a provisioning-operation TTL.
const CALL_DEADLINE: Duration = Duration::from_secs(120);

pub(crate) async fn create(client: &NodeClient, ask: Ask, events: &mpsc::Sender<crate::Event>) {
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
            ticket,
            machine: ask.machine,
            session: Box::new(info),
        }))
        .await
        .is_err()
    {
        return;
    }
    let FirstInput::Text(text) = ask.input else {
        let _closed = events
            .send(crate::Event::Creation(Answer::Complete(ticket)))
            .await;
        return;
    };
    let _closed = events
        .send(crate::Event::Creation(Answer::Progress {
            ticket,
            stage: Stage::InitialInput,
        }))
        .await;
    let answer = match deliver(client, session, &text).await {
        Ok(()) => Answer::Complete(ticket),
        Err(failure) => Answer::Failed {
            ticket,
            stage: Stage::InitialInput,
            failure,
        },
    };
    let _closed = events.send(crate::Event::Creation(answer)).await;
}

async fn register(client: &NodeClient, target: Target) -> Result<SessionInfo, Failure> {
    let register = async {
        match target {
            Target::New(spec) => client.create_session(spec).await,
            Target::Registered(session) => client.session(session).await,
        }
    };
    tokio::time::timeout(CALL_DEADLINE, register)
        .await
        .map_err(|_elapsed| Failure::OutcomeUnknown)?
        .map_err(|error| failure(&error))
}

async fn deliver(client: &NodeClient, session: SessionId, text: &str) -> Result<(), Failure> {
    let send = async {
        let content = vec![ContentBlock::Text {
            text: text.to_owned(),
        }];
        if client.protocol() >= ProtocolVersion::new(10) {
            client
                .prompt(session, content, Delivery::Start)
                .await
                .map(|_delivered| ())
        } else {
            client.send(session, SessionInput::Prompt { content }).await
        }
    };
    tokio::time::timeout(CALL_DEADLINE, send)
        .await
        .map_err(|_elapsed| Failure::OutcomeUnknown)?
        .map_err(|error| failure(&error))
}

/// Transport loss may hide a committed side effect; never turn it into retry.
fn failure(error: &CallError) -> Failure {
    match error {
        CallError::Remote(error) => Failure::Refused(error.code),
        CallError::NotInProtocol { .. }
        | CallError::UnassignedProtocol { .. }
        | CallError::RequestIdsExhausted => Failure::Unsupported,
        CallError::Disconnected | CallError::UnexpectedReply(_) => Failure::OutcomeUnknown,
    }
}
