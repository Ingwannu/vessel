//! Existing history/import calls run off the UI thread and never leak raw errors.
//! Own-history imports use session.create + resume (ADR 0053), not transfers.

pub use vessel_app_view::history::{Ask, Reply};

#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::history::{Batch, Outcome, Problem};

#[cfg(not(target_arch = "wasm32"))]
const READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(90);
#[cfg(not(target_arch = "wasm32"))]
const CREATE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(90);

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// The task retains its client and scope even when the sheet is dismissed.
    pub(crate) fn history(&self, ask: Ask) {
        let machine = match &ask {
            Ask::List { scope, .. } | Ask::Profiles { scope, .. } => &scope.machine,
            Ask::Import(batch) => &batch.scope.machine,
        };
        let client = self.client(machine.as_str()).cloned();
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let reply = match client {
                Err(_) => ask.refused(Problem::Offline),
                Ok(client) => answer(&client, ask).await,
            };
            let _closed = events.send(crate::Event::History(reply)).await;
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn answer(client: &vessel_client::NodeClient, ask: Ask) -> Reply {
    match ask {
        // Central integration supplies the existing agent.history contract.
        // Until then the typed boundary refuses rather than inventing a method.
        Ask::List { .. } => ask.refused(Problem::Unsupported),
        Ask::Profiles { scope, ticket } => {
            let result = tokio::time::timeout(
                READ_DEADLINE,
                client.call(vessel_proto::ListProfiles {
                    harness: Some(scope.harness.clone()),
                }),
            )
            .await
            .map_or(Err(Problem::Timeout), |result| result.map_err(read_problem));
            Reply::Profiles {
                scope,
                ticket,
                result,
            }
        }
        Ask::Import(batch) => import(client, batch).await,
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn import(client: &vessel_client::NodeClient, batch: Batch) -> Reply {
    if client.node().id != batch.node {
        return Ask::Import(batch).refused(Problem::IdentityChanged);
    }
    let mut outcomes = Vec::with_capacity(batch.entries.len());
    for entry in batch.entries {
        let result = tokio::time::timeout(
            CREATE_DEADLINE,
            client.create_session(entry.spec(&batch.scope)),
        )
        .await
        .map_or(Err(Problem::Uncertain), |result| {
            result.map(Box::new).map_err(create_problem)
        });
        outcomes.push(Outcome {
            id: entry.id,
            result,
        });
    }
    Reply::Imported {
        scope: batch.scope,
        ticket: batch.ticket,
        outcomes,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_problem(error: vessel_client::CallError) -> Problem {
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. } => Problem::Unsupported,
        vessel_client::CallError::UnexpectedReply(_)
        | vessel_client::CallError::RequestIdsExhausted => Problem::InvalidReply,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn create_problem(error: vessel_client::CallError) -> Problem {
    match error {
        vessel_client::CallError::Disconnected | vessel_client::CallError::UnexpectedReply(_) => {
            Problem::Uncertain
        }
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. } => Problem::Unsupported,
        vessel_client::CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "history/tests.rs"]
mod tests;
