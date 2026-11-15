//! Inbox's typed node boundary. Requests name an authenticated node as well as
//! its book route; answers cannot accidentally follow a reused machine name.
//! Existing durable inbox calls retain the node's request validation boundary.

use vessel_app_view::inbox::{Entry, Key, Pages, Problem, Read, Request, Snapshot, Source};
use vessel_app_view::machine_book::Name;
use vessel_proto::NodeId;
#[cfg(not(target_arch = "wasm32"))]
mod answer;
mod input;
#[cfg(not(target_arch = "wasm32"))]
mod node;
pub use input::Input;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod peer;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

/// A live authenticated source or an offline book route. A node identity
/// without its connection incarnation is never enough to authorize an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Current authenticated transport.
    Online(Source),
    /// Retain an unavailable machine instead of hiding its last snapshot.
    Offline(Name),
}
impl Target {
    /// Route shown even before a node is reachable.
    #[must_use]
    pub fn route(&self) -> &Name {
        match self {
            Self::Online(source) => &source.name,
            Self::Offline(route) => route,
        }
    }
    /// Authenticated node, absent for an offline route.
    #[must_use]
    pub fn node(&self) -> Option<NodeId> {
        match self {
            Self::Online(source) => Some(source.node),
            Self::Offline(_) => None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn connection_id() -> vessel_app_view::inbox::ConnectionId {
    vessel_app_view::inbox::ConnectionId::new(uuid::Uuid::new_v4().into_bytes())
}

/// User or refresh operation, never an untyped method name.
#[derive(Debug)]
pub enum Ask {
    /// Re-read which machines are connected.
    Discover,
    /// Read a bounded descending prefix of durable entries.
    List {
        /// Exact connection identity.
        source: Source,
        /// Explicitly requested prefix, kept across background refreshes.
        pages: Pages,
        /// Read identity supplied by the store.
        request: Request,
    },
    /// Confirm one live form before drawing actionable controls.
    Get {
        /// Exact connection identity.
        source: Source,
        /// Source record to confirm.
        key: Key,
        /// Detail request identity.
        request: Request,
    },
    /// Change shared read attention, never permission resolution.
    Mark {
        /// Exact connection identity.
        source: Source,
        /// Source record to mark.
        key: Key,
        /// New shared state.
        read: Read,
        /// Detail request identity.
        request: Request,
    },
    /// Send once and re-read the exact source; uncertain delivery is never
    /// retried automatically.
    Answer {
        /// Exact connection identity.
        source: Source,
        /// Source record whose actor request must still match.
        key: Key,
        /// Person's existing typed session answer.
        input: Input,
        /// Detail request identity.
        request: Request,
    },
}

/// The one event seam for discovery, background lists and live details.
#[derive(Debug)]
pub enum Answer {
    /// Live identities and offline routes, from the current registry.
    Targets(Result<Vec<Target>, Problem>),
    /// A node's current bounded snapshot.
    Listed(Source, Request, Result<Snapshot, Problem>),
    /// A confirmed form or withdrawal after a load, mark or answer.
    Detail(Key, Request, Result<Option<Entry>, Problem>),
}
impl Ask {
    fn validate(&self) -> Result<(), Problem> {
        match self {
            Self::Discover | Self::List { .. } => Ok(()),
            Self::Get { source, key, .. }
            | Self::Mark { source, key, .. }
            | Self::Answer { source, key, .. } => {
                if source.node == key.node {
                    Ok(())
                } else {
                    Err(Problem::Refused(vessel_proto::ErrorCode::InvalidRequest))
                }
            }
        }
    }

    /// Settle every refused command with its own ticket, including queue
    /// rejection and unsupported peers; no spinner waits on a lost request.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        match self {
            Self::Discover => Answer::Targets(Err(problem)),
            Self::List {
                source, request, ..
            } => Answer::Listed(source, request, Err(problem)),
            Self::Get { key, request, .. }
            | Self::Mark { key, request, .. }
            | Self::Answer { key, request, .. } => Answer::Detail(key, request, Err(problem)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn inbox_sources(&self, machines: &[vessel_app_view::Machine]) {
        let targets = machines
            .iter()
            .map(|machine| {
                let route = Name::parse(&machine.name).map_err(|_invalid| Problem::InvalidReply)?;
                Ok(self.inbox_target(route))
            })
            .collect();
        self.emit(crate::Event::Inbox(Answer::Targets(targets)))
            .await;
    }

    fn inbox_target(&self, route: Name) -> Target {
        if let Some(connected) = self.clients.get(route.as_str()) {
            Target::Online(Source {
                name: route,
                node: connected.client.node().id,
                connection: connected.inbox_connection,
            })
        } else {
            Target::Offline(route)
        }
    }

    pub(crate) async fn inbox(&self, ask: Ask) {
        if let Err(problem) = ask.validate() {
            self.emit(crate::Event::Inbox(ask.refused(problem))).await;
            return;
        }
        if let Ask::Discover = ask {
            let targets = self
                .saved_entries()
                .await
                .map_err(|_failure| Problem::InvalidReply)
                .and_then(|book| {
                    book.into_iter()
                        .map(|(route, _entry)| {
                            let route = Name::parse(route.as_str())
                                .map_err(|_invalid| Problem::InvalidReply)?;
                            Ok(self.inbox_target(route))
                        })
                        .collect()
                });
            self.emit(crate::Event::Inbox(Answer::Targets(targets)))
                .await;
            return;
        }
        let source = match &ask {
            Ask::Discover => return,
            Ask::List { source, .. }
            | Ask::Get { source, .. }
            | Ask::Mark { source, .. }
            | Ask::Answer { source, .. } => source,
        };
        let Some(connected) = self.clients.get(source.name.as_str()).filter(|connected| {
            connected.client.node().id == source.node
                && connected.inbox_connection == source.connection
        }) else {
            self.emit(crate::Event::Inbox(ask.refused(Problem::Offline)))
                .await;
            return;
        };
        let Ok(permit) = std::sync::Arc::clone(&self.inbox_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Inbox(
                ask.refused(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
            ))
            .await;
            return;
        };
        let client = connected.client.clone();
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let answer = bounded(&client, ask).await;
            let _closed = events.send(crate::Event::Inbox(answer)).await;
            drop(permit);
        }));
    }
}

/// The permit also covers the event-send wait, so a stopped view can retain
/// at most the named concurrency bound of completed replies.
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
const MAX_CALLS: usize = 8;
#[cfg(not(target_arch = "wasm32"))]
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

#[cfg(not(target_arch = "wasm32"))]
async fn bounded(client: &vessel_client::NodeClient, ask: Ask) -> Answer {
    let timeout = match &ask {
        Ask::Discover => {
            Answer::Targets(Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)))
        }
        Ask::List {
            source, request, ..
        } => Answer::Listed(
            source.clone(),
            *request,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
        Ask::Get { key, request, .. }
        | Ask::Mark { key, request, .. }
        | Ask::Answer { key, request, .. } => Answer::Detail(
            *key,
            *request,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
    };
    tokio::time::timeout(DEADLINE, call(client, ask))
        .await
        .unwrap_or(timeout)
}
#[cfg(not(target_arch = "wasm32"))]
async fn call(client: &vessel_client::NodeClient, ask: Ask) -> Answer {
    match ask {
        Ask::Discover => Answer::Targets(Err(Problem::InvalidReply)),
        Ask::List {
            source,
            pages,
            request,
        } => Answer::Listed(source, request, node::list(client, pages).await),
        Ask::Get { key, request, .. } => Answer::Detail(key, request, node::get(client, key).await),
        Ask::Mark {
            key, read, request, ..
        } => {
            let read = match read {
                Read::Read => vessel_proto::inbox::InboxRead::Read,
                Read::Unread => vessel_proto::inbox::InboxRead::Unread,
            };
            let result = match client.mark_inbox(node::wire_key(key), read).await {
                Ok(()) => node::get(client, key).await,
                Err(error) => Err(node::problem(error)),
            };
            Answer::Detail(key, request, result)
        }
        Ask::Answer {
            key,
            input,
            request,
            ..
        } => Answer::Detail(key, request, answer::send(client, key, input).await),
    }
}
