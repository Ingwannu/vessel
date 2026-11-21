//! Each connected machine's terminal list, followed for attention once the
//! machine admits claims (ADR 0089; `vessel_app_view::notices::terminals`
//! holds the Decision Log).
//!
//! A follower is one task per connection. It asks `terminal.list` with no
//! place, then again from the revision it read, which the node answers when
//! the list moves (ADR 0116, at most 30 s). What [`Watched`] finds goes to
//! the app as [`crate::Event::TerminalNotices`]. A newer connection to the
//! machine replaces the follower. A failed call ends it, as a lost
//! connection fails every call, and the next connection starts a new one.
//! A wait that runs past its bound asks again.
//!
//! While ADR 0089's constant is unassigned no machine admits claims, so no
//! follower starts.

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::AbortHandle;
use vessel_app_view::notices::claims::claims;
use vessel_app_view::notices::terminals::Watched;
use vessel_client::NodeClient;
use vessel_proto::notifications::ClearTerminalAttention;
use vessel_proto::terminals::ListTerminals;

use crate::{Event, State};

/// The longest one wait is given: the node's own bound (30 s) and a call's
/// (15 s), as the terminal tabs' list.
const WAIT: Duration = Duration::from_secs(30 + 15);

/// The followers running, one per machine.
#[derive(Debug, Default)]
pub(crate) struct Hub {
    following: HashMap<String, AbortHandle>,
}

#[cfg(test)]
impl Hub {
    /// Whether `machine`'s terminals are followed.
    pub(crate) fn follows(&self, machine: &str) -> bool {
        self.following.contains_key(machine)
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        for (_machine, follower) in self.following.drain() {
            follower.abort();
        }
    }
}

impl State {
    /// `machine` was connected afresh: follow its terminals if it admits
    /// claims, in place of an earlier connection's follower.
    pub(crate) fn terminal_notices_connected(&mut self, machine: &str) {
        if let Some(earlier) = self.terminal_notices.following.remove(machine) {
            earlier.abort();
        }
        let Some(client) = self
            .clients
            .get(machine)
            .map(|connected| connected.client.clone())
        else {
            return;
        };
        if !claims(Some(client.protocol())) {
            return;
        }
        let follower = tokio::spawn(follow(client, machine.to_owned(), self.events.clone()));
        let _replaced = self
            .terminal_notices
            .following
            .insert(machine.to_owned(), follower.abort_handle());
    }
}

impl State {
    /// Clear a viewed terminal's attention on `machine`, fire and forget:
    /// a lost clear leaves the attention, which the next view clears.
    pub(crate) fn clear_terminal_attention(&self, machine: &str, clear: ClearTerminalAttention) {
        let Some(client) = self
            .clients
            .get(machine)
            .map(|connected| connected.client.clone())
        else {
            return;
        };
        drop(tokio::spawn(async move {
            if let Err(error) = client.clear_terminal_attention(clear).await {
                tracing::debug!(%error, "a viewed terminal's attention was not cleared");
            }
        }));
    }
}

/// Follow `machine`'s terminal list until a call fails or the app closes.
pub(crate) async fn follow(client: NodeClient, machine: String, events: mpsc::Sender<Event>) {
    let mut watched = Watched::default();
    let mut changed_from = None;
    loop {
        let asked = client.call(ListTerminals {
            place: None,
            changed_from,
        });
        let list = match tokio::time::timeout(WAIT, asked).await {
            Err(_elapsed) => continue,
            Ok(Err(error)) => {
                tracing::debug!(%error, "a machine's terminals are no longer followed");
                return;
            }
            Ok(Ok(list)) => list,
        };
        changed_from = Some(list.revision);
        let notices = watched.read(&machine, &list.terminals);
        if notices.is_empty() {
            continue;
        }
        let told = Event::TerminalNotices {
            machine: machine.clone(),
            notices,
        };
        if events.send(told).await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
