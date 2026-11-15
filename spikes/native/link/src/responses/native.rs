//! One bounded response deadline: uncertainty never means an automatic denial.
use super::{Answer, Ask, Response};
use crate::connections::{RequestFailure, SelectedConnection};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::connection::Refusal;
use vessel_app_view::responses::{MAX_PENDING, Problem};
use vessel_client::CallError;
use vessel_proto::{PermissionOutcome, SessionInput};

/// Paseo's response acknowledgement wait, not a human-answer expiry.
const RESPONSE_DEADLINE: Duration = Duration::from_secs(15);

#[derive(Debug)]
pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_PENDING)))
    }
}

impl Hub {
    pub(super) async fn ask(
        &self,
        connection: Result<SelectedConnection, Refusal>,
        ask: Ask,
        events: mpsc::Sender<crate::Event>,
    ) {
        let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
            // Refusal uses the actor's backpressure; spawning here would make
            // a full response queue create an unbounded queue of refusal tasks.
            let _closing = events
                .send(crate::Event::Responded(ask.refused(Problem::Busy)))
                .await;
            return;
        };
        drop(tokio::spawn(async move {
            let answer = match connection {
                Ok(connection) => run(&connection, ask).await,
                Err(_changed) => ask.refused(Problem::Offline),
            };
            let _closing = events.send(crate::Event::Responded(answer)).await;
            drop(permit);
        }));
    }
}

async fn run(connection: &SelectedConnection, ask: Ask) -> Answer {
    let input = match ask.response {
        Response::Permission { id, option } => SessionInput::AnswerPermission {
            id,
            outcome: PermissionOutcome::Selected { option },
        },
        Response::Question { id, answer } => SessionInput::AnswerQuestion { id, answer },
    };
    let result = match tokio::time::timeout(
        RESPONSE_DEADLINE,
        connection.call(vessel_proto::SendInput {
            session: ask.pending.target.session,
            input,
        }),
    )
    .await
    {
        Ok(result) => result.map(|_ack| ()).map_err(|failure| match failure {
            RequestFailure::Refused(Refusal::Changed | Refusal::Closed) => Problem::Unconfirmed,
            RequestFailure::Call(error) => problem(error),
        }),
        Err(_unconfirmed) => Err(Problem::Unconfirmed),
    };
    Answer {
        pending: ask.pending,
        result,
    }
}

fn problem(error: CallError) -> Problem {
    match error {
        CallError::Disconnected => Problem::Unconfirmed,
        CallError::UnexpectedReply(_) => Problem::Invalid,
        CallError::RequestIdsExhausted => Problem::Exhausted,
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => {
            Problem::Unsupported
        }
        CallError::Remote(error) => Problem::Refused(error.code),
    }
}

#[cfg(test)]
mod tests;
