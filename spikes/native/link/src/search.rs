//! Searching one machine's messages (`session.search`, protocol 12, ADR
//! 0037): the app asks every machine it reaches and merges their pages
//! (`view::search`); each machine's answer comes back on its own, tagged
//! with the search it belongs to.
//!
//! [Decision Log]
//! - Purpose: the palette's Messages group lists what was said in any
//!   session on any machine; a node answers only for itself.
//! - Choice: one call per machine on a task of its own, so a slow or
//!   unreachable machine holds back no other; the answer carries the
//!   search's ticket, and the app drops answers to a search the person has
//!   already changed.
//! - A machine older than protocol 12 is not asked at all (the client
//!   refuses the call before sending it): it answers [`Refusal::Older`],
//!   which the palette says once, not as an error.

#[cfg(not(target_arch = "wasm32"))]
use vessel_client::CallError;
use vessel_proto::ErrorCode;
use vessel_proto::search::{SearchMessages, SearchPage};

#[cfg(not(target_arch = "wasm32"))]
use crate::{Event, LinkError, State};

/// One machine's page of a search.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine.
    pub machine: String,
    /// The search it belongs to (the app's count of searches started).
    pub ticket: u64,
    /// The words, and the page.
    pub request: SearchMessages,
}

/// Why a machine gave no page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It speaks a protocol before 12: it cannot search.
    Older,
    /// It is not connected.
    NotConnected,
    /// It answered with an error, or the connection failed.
    Failed {
        /// The node's code; `None` when the connection failed.
        code: Option<ErrorCode>,
        /// Its words.
        message: String,
    },
}

/// A machine's answer to [`Ask`].
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine.
    pub machine: String,
    /// The search it belongs to.
    pub ticket: u64,
    /// Its page, or why there is none.
    pub result: Result<SearchPage, Refusal>,
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    /// Ask `ask.machine` for a page, on a task of its own; the answer
    /// comes back as [`Event::Searched`].
    pub(crate) fn search(&self, ask: Ask) {
        let events = self.events.clone();
        let client = self
            .clients
            .get(&ask.machine)
            .map(|connected| connected.client.clone());
        drop(tokio::spawn(async move {
            let result = if let Some(client) = client {
                client
                    .call(ask.request)
                    .await
                    .map_err(|error| refusal(&error))
            } else {
                tracing::debug!(
                    error = %LinkError::NotConnected(ask.machine.clone()),
                    "a search found no connection"
                );
                Err(Refusal::NotConnected)
            };
            let answer = Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result,
            };
            if events.send(Event::Searched(answer)).await.is_err() {
                tracing::debug!("a search answered after the app closed");
            }
        }));
    }
}

/// Why a call gave no page: a machine too old to search, or its error.
#[cfg(not(target_arch = "wasm32"))]
fn refusal(error: &CallError) -> Refusal {
    if let CallError::NotInProtocol { .. } = error {
        return Refusal::Older;
    }
    Refusal::Failed {
        code: error.code(),
        message: error.to_string(),
    }
}
