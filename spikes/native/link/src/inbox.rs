//! Inbox's typed node boundary. Requests name an authenticated node as well as
//! its book route; answers cannot accidentally follow a reused machine name.
//! The existing durable inbox calls are wired after the central node merge.

use vessel_app_view::inbox::{Entry, Key, Pages, Problem, Read, Request, Snapshot, Source};
use vessel_app_view::machine_book::Name;
use vessel_proto::NodeId;
mod input;
pub use input::Input;

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
        match ask {
            Ask::Discover => {
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
            }
            // The app's current node contract predates durable attention.
            // The central integration replaces this explicit version refusal
            // with the existing typed calls, never a guessed wire method.
            ask @ (Ask::List { .. } | Ask::Get { .. } | Ask::Mark { .. } | Ask::Answer { .. }) => {
                self.emit(crate::Event::Inbox(
                    ask.refused(Problem::Refused(vessel_proto::ErrorCode::Unsupported)),
                ))
                .await;
            }
        }
    }
}
