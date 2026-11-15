//! Registration and first delivery have separate failure boundaries.

use std::time::Duration;

use tokio::sync::mpsc;
use vessel_app_view::creation::{Failure, Stage};
use vessel_client::{CallError, NodeClient};
use vessel_proto::{ContentBlock, Delivery, ProtocolVersion, SessionInput};

use super::{Answer, Ask, FirstInput};

/// A legacy single RPC has a bound; this is not a provisioning-operation TTL.
const CALL_DEADLINE: Duration = Duration::from_secs(120);

pub(super) async fn create(client: &NodeClient, ask: Ask, events: &mpsc::Sender<crate::Event>) {
    let ticket = ask.ticket;
    let created = tokio::time::timeout(CALL_DEADLINE, client.create_session(ask.spec)).await;
    let info = match created {
        Ok(Ok(info)) => info,
        result => {
            let failure = match result {
                Ok(Err(error)) => failure(&error),
                Err(_elapsed) => Failure::OutcomeUnknown,
                Ok(Ok(_)) => return,
            };
            let _closed = events
                .send(crate::Event::Creation(Answer::Failed {
                    ticket,
                    stage: Stage::Resource,
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
    let send = async {
        let content = vec![ContentBlock::Text {
            text: text.to_string(),
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
    let answer = match tokio::time::timeout(CALL_DEADLINE, send).await {
        Ok(Ok(())) => Answer::Complete(ticket),
        Ok(Err(error)) => Answer::Failed {
            ticket,
            stage: Stage::InitialInput,
            failure: failure(&error),
        },
        Err(_elapsed) => Answer::Failed {
            ticket,
            stage: Stage::InitialInput,
            failure: Failure::OutcomeUnknown,
        },
    };
    let _closed = events.send(crate::Event::Creation(answer)).await;
}

/// Transport loss may hide a committed side effect; never turn it into retry.
fn failure(error: &CallError) -> Failure {
    match error {
        CallError::Remote(error) => Failure::Refused(error.code),
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. }
        | CallError::RequestIdsExhausted => Failure::Unsupported,
        CallError::Disconnected | CallError::UnexpectedReply(_) => Failure::OutcomeUnknown,
    }
}
