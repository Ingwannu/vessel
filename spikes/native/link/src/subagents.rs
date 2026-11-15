//! Registered subagent calls, scoped by the node that actually answered welcome.

pub use vessel_app_view::subagents::{SubagentAsk as Ask, SubagentReply as Reply};

#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::subagents::TrackProblem;
#[cfg(not(target_arch = "wasm32"))]
const READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);
#[cfg(not(target_arch = "wasm32"))]
const DETACH_DEADLINE: std::time::Duration = std::time::Duration::from_secs(90);

/// Match the shared Devices bound: navigation cannot grow background RPC tasks.
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
    pub(crate) async fn subagents(&self, ask: Ask) {
        let selected =
            if ask.context().node() == ask.node() && ask.context().machine() == ask.machine() {
                self.selected_connection(ask.context())
                    .map_err(|_retired| TrackProblem::IdentityChanged)
            } else {
                Err(TrackProblem::IdentityChanged)
            };
        let client = match selected {
            Ok(client) => client,
            Err(problem) => {
                self.emit(crate::Event::Subagents(ask.refused(problem)))
                    .await;
                return;
            }
        };
        let Ok(permit) = std::sync::Arc::clone(&self.subagent_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Subagents(ask.refused(TrackProblem::Busy)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let reply = answer(&client, ask).await;
            let _closed = tokio::time::timeout(
                PUBLICATION_DEADLINE,
                events.send(crate::Event::Subagents(reply)),
            )
            .await;
            drop(permit);
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn answer(client: &crate::connections::SelectedConnection, ask: Ask) -> Reply {
    use vessel_proto::subagents::{
        ChildLimit, DetachSession, ListProviderChildren, ReadProviderChild,
    };
    if ask.context().node() != ask.node() || ask.context().machine() != ask.machine() {
        return ask.refused(TrackProblem::IdentityChanged);
    }
    // Current-parent metadata is unsafe to infer from immutable Creator when
    // the peer cannot project the durable detach receipt.
    if vessel_proto::subagents::SUBAGENT_RELATIONS.is_none_or(|since| since > client.protocol()) {
        return ask.refused(TrackProblem::Unsupported);
    }
    match ask {
        Ask::Lookup { target, ticket } => {
            let result = read(client.call(vessel_proto::GetSession {
                session: target.address.session,
            }))
            .await
            .and_then(|info| {
                if info.id == target.address.session && info.parentage().is_ok() {
                    Ok(Box::new(info))
                } else {
                    Err(TrackProblem::InvalidReply)
                }
            });
            Reply::LookedUp {
                target,
                ticket,
                result,
            }
        }
        Ask::Managed {
            host,
            parent,
            ticket,
        } => {
            let result = read(client.call(vessel_proto::ListSessions {}))
                .await
                .and_then(|rows| {
                    if rows.len() > vessel_app_view::subagents::MAX_MANAGED_SNAPSHOTS
                        || rows.iter().any(|info| info.parentage().is_err())
                    {
                        Err(TrackProblem::InvalidReply)
                    } else {
                        Ok(rows)
                    }
                });
            Reply::Managed {
                host,
                parent,
                ticket,
                result,
            }
        }
        Ask::Providers(read_request) => {
            let result = read(client.call(ListProviderChildren {
                session: read_request.target.address.session,
            }))
            .await;
            Reply::Providers {
                read: read_request,
                result,
            }
        }
        Ask::Timeline(read_request) => {
            let request = ReadProviderChild {
                session: read_request.query.target.address.session,
                child: read_request.query.child.clone(),
                cursor: read_request.cursor,
                limit: ChildLimit::default(),
            };
            let result = read(client.call(request)).await;
            Reply::Timeline {
                read: read_request,
                result,
            }
        }
        Ask::Detach { choice, ticket } => {
            let result = tokio::time::timeout(
                DETACH_DEADLINE,
                client.call(DetachSession {
                    session: choice.child.address.session,
                }),
            )
            .await
            .map_or(Err(TrackProblem::Uncertain), |result| {
                result.map_err(mutation_failure)
            })
            .and_then(|result| validate_detach(&choice, result));
            Reply::Detached {
                choice,
                ticket,
                result,
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
async fn read<T>(
    operation: impl Future<Output = Result<T, crate::connections::RequestFailure>>,
) -> Result<T, TrackProblem> {
    tokio::time::timeout(READ_DEADLINE, operation)
        .await
        .map_or(Err(TrackProblem::Timeout), |result| {
            result.map_err(read_failure)
        })
}
#[cfg(not(target_arch = "wasm32"))]
fn validate_detach(
    choice: &vessel_app_view::subagents::DetachChoice,
    result: vessel_proto::subagents::DetachedSession,
) -> Result<vessel_proto::subagents::DetachedSession, TrackProblem> {
    if result.session.id != choice.child.address.session
        || result.receipt.former_parent() != choice.parent
        || result.session.detach_receipt != Some(result.receipt)
        || result.session.parentage().is_err()
    {
        return Err(TrackProblem::Uncertain);
    }
    Ok(result)
}
#[cfg(not(target_arch = "wasm32"))]
fn read_failure(error: crate::connections::RequestFailure) -> TrackProblem {
    match error {
        crate::connections::RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Changed,
        ) => TrackProblem::IdentityChanged,
        crate::connections::RequestFailure::Refused(
            vessel_app_view::connection::Refusal::Closed,
        ) => TrackProblem::Offline,
        crate::connections::RequestFailure::Call(error) => call_failure(error),
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn call_failure(error: vessel_client::CallError) -> TrackProblem {
    match error {
        vessel_client::CallError::Disconnected => TrackProblem::Offline,
        vessel_client::CallError::Remote(error) => TrackProblem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => TrackProblem::Unsupported,
        vessel_client::CallError::UnexpectedReply(_) => TrackProblem::InvalidReply,
        vessel_client::CallError::RequestIdsExhausted => TrackProblem::Exhausted,
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn mutation_failure(error: crate::connections::RequestFailure) -> TrackProblem {
    match error {
        crate::connections::RequestFailure::Refused(_)
        | crate::connections::RequestFailure::Call(
            vessel_client::CallError::Disconnected | vessel_client::CallError::UnexpectedReply(_),
        ) => TrackProblem::Uncertain,
        other @ crate::connections::RequestFailure::Call(_) => read_failure(other),
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
#[path = "subagents/tests.rs"]
mod tests;
