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
use crate::{Event, State};
use vessel_app_view::connection::ConnectionContext;

/// One machine's page of a search.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine.
    pub machine: String,
    /// Shared immutable admission captured when this query started.
    pub connection: Option<ConnectionContext>,
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
    /// The bounded read did not finish.
    TimedOut,
    /// It answered with an error, or the connection failed.
    Failed {
        /// The node's code; `None` when the connection failed.
        code: Option<ErrorCode>,
    },
}

/// A machine's answer to [`Ask`].
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine.
    pub machine: String,
    /// Exact original query admission, including on a local preflight refusal.
    pub connection: Option<ConnectionContext>,
    /// Weak actor-issued validity; absent only on a local refusal or demo result.
    pub delivery: Option<crate::DeliveryGuard>,
    /// The search it belongs to.
    pub ticket: u64,
    /// Its page, or why there is none.
    pub result: Result<SearchPage, Refusal>,
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    /// Use only the query's original admitted client, never a later alias target.
    pub(crate) fn search(&self, ask: Ask) {
        let events = self.events.clone();
        let client = ask
            .connection
            .as_ref()
            .filter(|context| context.machine().as_str() == ask.machine)
            .ok_or(vessel_app_view::connection::Refusal::Changed)
            .and_then(|context| self.selected_connection(context));
        drop(tokio::spawn(async move {
            let Ask {
                machine,
                connection,
                ticket,
                request,
            } = ask;
            let local_failure = || Answer {
                machine: machine.clone(),
                connection: connection.clone(),
                delivery: None,
                ticket,
                result: Err(Refusal::NotConnected),
            };
            let Ok(client) = client else {
                let _closed = events.send(Event::Searched(local_failure())).await;
                return;
            };
            let delivery = client.delivery_guard();
            let result = bounded(client.call(request), DEADLINE).await;
            let answer = Answer {
                machine: machine.clone(),
                connection: connection.clone(),
                delivery: Some(delivery),
                ticket,
                result,
            };
            if client
                .send_guarded(&events, Event::Searched(answer))
                .await
                .is_err()
            {
                // No selected client is retained while a local refusal waits for UI room.
                drop(client);
                let _closed = events.send(Event::Searched(local_failure())).await;
            }
        }));
    }
}

/// A slow peer must settle its machine slot so other results remain usable.
#[cfg(not(target_arch = "wasm32"))]
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

#[cfg(not(target_arch = "wasm32"))]
async fn bounded(
    call: impl std::future::Future<Output = Result<SearchPage, crate::connections::RequestFailure>>,
    deadline: std::time::Duration,
) -> Result<SearchPage, Refusal> {
    match tokio::time::timeout(deadline, call).await {
        Ok(answer) => answer.map_err(|error| match error {
            crate::connections::RequestFailure::Refused(_why) => Refusal::NotConnected,
            crate::connections::RequestFailure::Call(error) => refusal(&error),
        }),
        Err(_elapsed) => Err(Refusal::TimedOut),
    }
}

/// Project structured codes only: peer text may contain sensitive connection data.
#[cfg(not(target_arch = "wasm32"))]
fn refusal(error: &CallError) -> Refusal {
    match error {
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => Refusal::Older,
        CallError::Disconnected => Refusal::NotConnected,
        CallError::Remote(_) | CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted => {
            Refusal::Failed { code: error.code() }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
