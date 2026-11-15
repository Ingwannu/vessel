//! Existing history/import calls run off the UI thread and never leak raw errors.
//! Own-history imports use session.create + resume (ADR 0053), not transfers.

pub use vessel_app_view::history::{Ask, Reply};

#[cfg(not(target_arch = "wasm32"))]
mod activate;

#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::history::{Batch, Cursor, Entry, Listed, Outcome, Problem};

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
        Ask::List {
            scope,
            ticket,
            cursor,
        } => {
            let request = vessel_proto::history::GetAgentHistory {
                harness: scope.harness.clone(),
                profile: scope.profile.clone(),
                cwd: scope.cwd.clone(),
                cursor: cursor.map(|cursor| cursor.as_str().to_owned()),
            };
            let result = tokio::time::timeout(READ_DEADLINE, client.call(request))
                .await
                .map_or(Err(Problem::Timeout), |result| result.map_err(read_problem))
                .and_then(|page| project(client.node().id, page));
            Reply::Listed {
                scope,
                ticket,
                result,
            }
        }
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
fn project(
    node: vessel_proto::NodeId,
    page: vessel_proto::history::AgentHistoryPage,
) -> Result<Listed, Problem> {
    if page.conversations.len() > vessel_proto::history::AgentHistoryPage::MAX_ENTRIES {
        return Err(Problem::InvalidReply);
    }
    let next = page.next.map(Cursor::parse).transpose()?;
    let entries = page
        .conversations
        .into_iter()
        .map(|entry| Entry {
            id: entry.harness_session,
            cwd: entry.cwd,
            title: entry.title,
            updated_at: entry.updated_at,
            in_vessel: entry.in_vessel,
        })
        .collect();
    Ok(Listed {
        entries,
        next,
        node,
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn import(client: &vessel_client::NodeClient, batch: Batch) -> Reply {
    if client.node().id != batch.node {
        return Ask::Import(batch).refused(Problem::IdentityChanged);
    }
    let mut outcomes = Vec::with_capacity(batch.entries.len());
    for entry in batch.entries {
        let result = match entry.in_vessel {
            Some(session) => activate::existing(client, &batch.scope, &entry, session).await,
            None => tokio::time::timeout(
                CREATE_DEADLINE,
                client.create_session(entry.spec(&batch.scope)),
            )
            .await
            .map_or(Err(Problem::Uncertain), |result| {
                result.map(Box::new).map_err(create_problem)
            }),
        };
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
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => Problem::Unsupported,
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
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => Problem::Unsupported,
        vessel_client::CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
#[path = "history/tests.rs"]
pub(crate) mod tests;
