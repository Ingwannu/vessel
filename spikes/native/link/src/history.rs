//! Existing history/import calls run off the UI thread and never leak raw errors.
//! Own-history imports use session.create + resume (ADR 0053), not transfers.

mod delivery;
pub use delivery::Answer;

pub use vessel_app_view::history::{Ask, Reply};

#[cfg(not(target_arch = "wasm32"))]
mod activate;

#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::history::{Batch, Cursor, Entry, Listed, Outcome, Problem};

#[cfg(not(target_arch = "wasm32"))]
const READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(90);
/// A ranked listing (ADR 0109) may wait up to 90 s for one of the node's
/// places and then take up to 90 s (its loads' budget and the agent).
#[cfg(not(target_arch = "wasm32"))]
const RANKED_DEADLINE: std::time::Duration = std::time::Duration::from_secs(180);
#[cfg(not(target_arch = "wasm32"))]
const CREATE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(90);

#[cfg(not(target_arch = "wasm32"))]
const MAX_CALLS: usize = 8;
#[cfg(not(target_arch = "wasm32"))]
const PUBLICATION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct Hub(std::sync::Arc<tokio::sync::Semaphore>);
#[cfg(not(target_arch = "wasm32"))]
impl Default for Hub {
    fn default() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CALLS)))
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Every effect retains its original admission; no alias re-resolution in a task.
    pub(crate) async fn history(&self, ask: Ask) {
        let scope = ask.scope();
        let selected = if scope.machine == *scope.context.machine() {
            self.selected_connection(&scope.context)
                .map_err(|_retired| Problem::IdentityChanged)
        } else {
            Err(Problem::IdentityChanged)
        };
        let client = match selected {
            Ok(client) => client,
            Err(problem) => {
                self.emit(crate::Event::History(Answer::refused(ask, problem)))
                    .await;
                return;
            }
        };
        let Ok(permit) = std::sync::Arc::clone(&self.history_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::History(Answer::refused(ask, Problem::Unsent)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let original = ask.clone();
            let guard = client.delivery_guard();
            let reply = answer(&client, ask).await;
            delivery::publish(&client, &events, original, reply, guard).await;
            drop(permit);
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn answer(client: &crate::connections::SelectedConnection, ask: Ask) -> Reply {
    if matches!(&ask, Ask::List { .. } | Ask::Import(_))
        && vessel_proto::Method::GetAgentHistory
            .since()
            .is_none_or(|since| since > client.protocol())
    {
        return ask.refused(Problem::Unsupported);
    }
    match ask {
        Ask::List {
            scope,
            ticket,
            cursor,
            ranked,
        } => {
            let deadline = if ranked.is_some() {
                RANKED_DEADLINE
            } else {
                READ_DEADLINE
            };
            let (limit, query) =
                ranked.map_or((None, None), |ranked| (Some(ranked.limit), ranked.query));
            let request = vessel_proto::history::GetAgentHistory {
                harness: scope.harness.clone(),
                profile: scope.profile.clone(),
                cwd: scope.cwd.clone(),
                cursor: cursor.map(|cursor| cursor.as_str().to_owned()),
                limit,
                query,
            };
            let result = tokio::time::timeout(deadline, client.call(request))
                .await
                .map_or(Err(Problem::Timeout), |result| result.map_err(read_failure))
                .and_then(|page| project(scope.context.node(), page));
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
            .map_or(Err(Problem::Timeout), |result| result.map_err(read_failure));
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
            first_prompt: entry.first_prompt,
            last_prompt: entry.last_prompt,
        })
        .collect();
    Ok(Listed {
        entries,
        next,
        node,
        already_imported: page.already_imported.unwrap_or(0),
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn import(client: &crate::connections::SelectedConnection, batch: Batch) -> Reply {
    if batch.scope.context.node() != batch.node {
        return Ask::Import(batch).refused(Problem::IdentityChanged);
    }
    if batch.entries.len() > vessel_app_view::history::MAX_IMPORT_ENTRIES {
        return Ask::Import(batch).refused(Problem::Capacity);
    }
    let mut outcomes = Vec::with_capacity(batch.entries.len());
    for entry in batch.entries {
        let result = match entry.in_vessel {
            Some(session) => activate::existing(client, &batch.scope, &entry, session).await,
            None => tokio::time::timeout(
                CREATE_DEADLINE,
                client.call(vessel_proto::CreateSession {
                    spec: entry.spec(&batch.scope),
                }),
            )
            .await
            .map_or(Err(Problem::Uncertain), |result| {
                result.map_err(create_failure).and_then(|info| {
                    activate::created(&info, &batch.scope, &entry)?;
                    Ok(Box::new(info))
                })
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

#[cfg(not(target_arch = "wasm32"))]
fn read_failure(error: crate::connections::RequestFailure) -> Problem {
    match error {
        crate::connections::RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Changed,
        ) => Problem::IdentityChanged,
        crate::connections::RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Closed,
        ) => Problem::Offline,
        crate::connections::RequestFailure::Call(error) => read_problem(error),
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn create_failure(error: crate::connections::RequestFailure) -> Problem {
    match error {
        crate::connections::RequestFailure::Refused(_) => Problem::Uncertain,
        crate::connections::RequestFailure::Call(error) => create_problem(error),
    }
}
