//! Inbox's typed node boundary. Requests name an authenticated node as well as
//! its book route; answers cannot accidentally follow a reused machine name.
//! Existing durable inbox calls retain the node's request validation boundary.

use vessel_app_view::inbox::{Entry, Key, Pages, Problem, Read, Request, Snapshot, Source};
use vessel_app_view::machine_book::Name;
use vessel_proto::NodeId;
#[cfg(not(target_arch = "wasm32"))]
mod admission;
#[cfg(not(target_arch = "wasm32"))]
mod answer;
mod input;
#[cfg(not(target_arch = "wasm32"))]
mod node;
#[cfg(not(target_arch = "wasm32"))]
mod presence;
#[cfg(not(target_arch = "wasm32"))]
mod reading;
pub mod terminals;
#[cfg(not(target_arch = "wasm32"))]
mod transport;
pub use input::Input;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use presence::PresenceHub;
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
            Self::Online(source) => source.name(),
            Self::Offline(route) => route,
        }
    }
    /// Authenticated node, absent for an offline route.
    #[must_use]
    pub fn node(&self) -> Option<NodeId> {
        match self {
            Self::Online(source) => Some(source.node()),
            Self::Offline(_) => None,
        }
    }
}

/// User or refresh operation, never an untyped method name.
#[derive(Debug)]
pub enum Ask {
    /// Re-read which machines are connected.
    Discover,
    /// Operate on one exact terminal attention revision.
    Terminal {
        /// Original authenticated admission.
        source: Source,
        /// Exact target and requested operation.
        action: terminals::Action,
        /// Non-reused local response identity.
        request: Request,
    },
    /// Coalesced real activity or focus report for this exact transport.
    Presence {
        /// Authenticated transport receiving the report.
        source: Source,
        /// No text, coordinates or client timestamp crosses this boundary.
        report: vessel_app_view::inbox::presence::Report,
    },
    /// Passively clear only the observed session watermark after checking live requests.
    ReadSession {
        /// Exact authenticated source.
        source: Source,
        /// Session actually viewed.
        session: vessel_proto::SessionId,
        /// Exclusive observed watermark.
        through: vessel_proto::EventSeq,
        /// Correlation identity for safe late failure feedback.
        request: Request,
    },
    /// Read a bounded descending prefix of durable entries.
    List {
        /// Exact connection identity.
        source: Source,
        /// Explicitly requested prefix, kept across background refreshes.
        pages: Pages,
        /// Read identity supplied by the store.
        request: Request,
    },
    /// Claim one exact new source; no client-supplied recipient identity.
    Claim(vessel_app_view::inbox::admission::Pending),
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
    /// Original admission, exact action and correlated terminal completion.
    Terminal(
        Source,
        terminals::Action,
        Request,
        Result<terminals::ResultState, Problem>,
    ),
    /// Safe status of a connection's last presence report.
    Presence(Source, Result<(), Problem>),
    /// Completion of the exact passive session read.
    ReadSession(
        Source,
        vessel_proto::SessionId,
        Request,
        Result<(), Problem>,
    ),
    /// A node's current bounded snapshot.
    Listed(Source, Request, Result<Snapshot, Problem>),
    /// A claim result, correlated to its exact source/transport/ticket.
    Claimed(
        Source,
        Key,
        Request,
        Result<vessel_app_view::inbox::admission::Admission, Problem>,
    ),
    /// A confirmed form or withdrawal after a load, mark or answer.
    Detail(Source, Key, Request, Result<Option<Entry>, Problem>),
}
impl Ask {
    #[cfg(not(target_arch = "wasm32"))]
    fn validate(&self) -> Result<(), Problem> {
        match self {
            Self::Claim(pending) => {
                if pending.source.node() == pending.key.node {
                    Ok(())
                } else {
                    Err(Problem::InvalidReply)
                }
            }
            Self::Terminal { .. }
            | Self::Discover
            | Self::List { .. }
            | Self::Presence { .. }
            | Self::ReadSession { .. } => Ok(()),
            Self::Get { source, key, .. }
            | Self::Mark { source, key, .. }
            | Self::Answer { source, key, .. } => {
                if source.node() == key.node {
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
            Self::Claim(pending) => {
                Answer::Claimed(pending.source, pending.key, pending.request, Err(problem))
            }
            Self::Discover => Answer::Targets(Err(problem)),
            Self::Terminal {
                source,
                action,
                request,
            } => Answer::Terminal(source, action, request, Err(problem)),
            Self::Presence { source, .. } => Answer::Presence(source, Err(problem)),
            Self::ReadSession {
                source,
                session,
                request,
                ..
            } => Answer::ReadSession(source, session, request, Err(problem)),
            Self::List {
                source, request, ..
            } => Answer::Listed(source, request, Err(problem)),
            Self::Get {
                source,
                key,
                request,
            }
            | Self::Mark {
                source,
                key,
                request,
                ..
            }
            | Self::Answer {
                source,
                key,
                request,
                ..
            } => Answer::Detail(source, key, request, Err(problem)),
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
        if let Some(connected) = self.clients.get(route.as_str())
            && connected.matches(&connected.context)
            && let Ok(source) = Source::new(connected.context.clone())
        {
            Target::Online(source)
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
            Ask::Claim(pending) => &pending.source,
            Ask::Terminal { source, .. }
            | Ask::ReadSession { source, .. }
            | Ask::Presence { source, .. }
            | Ask::List { source, .. }
            | Ask::Get { source, .. }
            | Ask::Mark { source, .. }
            | Ask::Answer { source, .. } => source,
        };
        let selected = match self.selected_connection(source.context()) {
            Ok(selected) => selected,
            Err(_refused) => {
                self.emit(crate::Event::Inbox(ask.refused(Problem::Offline)))
                    .await;
                return;
            }
        };
        let Some(connected) = self.clients.get(source.name().as_str()) else {
            self.emit(crate::Event::Inbox(ask.refused(Problem::Offline)))
                .await;
            return;
        };
        let client = transport::Client::new(
            selected,
            source.context().clone(),
            connected.client.protocol(),
        );
        if let Ask::Presence { source, report } = ask {
            connected
                .inbox_presence
                .report(client, source, report, self.events.clone());
            return;
        }
        let Ok(permit) = std::sync::Arc::clone(&self.inbox_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Inbox(
                ask.refused(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
            ))
            .await;
            return;
        };
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
async fn bounded(client: &transport::Client, ask: Ask) -> Answer {
    let timeout = match &ask {
        Ask::Terminal {
            source,
            action,
            request,
        } => Answer::Terminal(
            source.clone(),
            *action,
            *request,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
        Ask::Claim(pending) => Answer::Claimed(
            pending.source.clone(),
            pending.key,
            pending.request,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
        Ask::ReadSession {
            source,
            session,
            request,
            ..
        } => Answer::ReadSession(
            source.clone(),
            *session,
            *request,
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
        Ask::Presence { source, .. } => Answer::Presence(
            source.clone(),
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
        ),
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
        Ask::Get {
            source,
            key,
            request,
        }
        | Ask::Mark {
            source,
            key,
            request,
            ..
        }
        | Ask::Answer {
            source,
            key,
            request,
            ..
        } => Answer::Detail(
            source.clone(),
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
async fn call(client: &transport::Client, ask: Ask) -> Answer {
    match ask {
        Ask::Terminal {
            source,
            action,
            request,
        } => Answer::Terminal(source, action, request, action.run(client).await),
        Ask::Claim(pending) => {
            let result = admission::claim(client, &pending).await;
            Answer::Claimed(pending.source, pending.key, pending.request, result)
        }
        Ask::ReadSession {
            source,
            session,
            through,
            request,
        } => Answer::ReadSession(
            source,
            session,
            request,
            reading::clear(client, session, through).await,
        ),
        Ask::Discover => Answer::Targets(Err(Problem::InvalidReply)),
        Ask::Presence { source, .. } => Answer::Presence(source, Err(Problem::InvalidReply)),
        Ask::List {
            source,
            pages,
            request,
        } => Answer::Listed(source, request, node::list(client, pages).await),
        Ask::Get {
            source,
            key,
            request,
        } => Answer::Detail(source, key, request, node::get(client, key).await),
        Ask::Mark {
            source,
            key,
            read,
            request,
        } => {
            let read = match read {
                Read::Read => vessel_proto::inbox::InboxRead::Read,
                Read::Unread => vessel_proto::inbox::InboxRead::Unread,
            };
            let result = match client
                .call(vessel_proto::inbox::MarkInbox {
                    key: node::wire_key(key),
                    read,
                })
                .await
            {
                Ok(_ack) => node::get(client, key).await,
                Err(error) => Err(node::problem(error)),
            };
            Answer::Detail(source, key, request, result)
        }
        Ask::Answer {
            source,
            key,
            input,
            request,
        } => Answer::Detail(source, key, request, answer::send(client, key, input).await),
    }
}
