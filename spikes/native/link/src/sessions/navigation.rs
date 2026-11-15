//! One bounded navigation task waits for authenticated connection snapshots.
//! A watch channel coalesces reconnects; superseding requests cancel the old read.

use super::{Answer, Ask, Failure, Located};
use crate::{Event, State};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, watch};
use vessel_app_view::Card;
use vessel_app_view::machine_book::Name;
use vessel_app_view::sessions::{NavigationTarget, SessionAddress};
use vessel_client::NodeClient;

/// Includes connection readiness and the metadata read, not one budget per retry.
pub(super) const LOOKUP_DEADLINE: Duration = Duration::from_secs(10);
#[derive(Clone, Debug)]
struct Connection {
    host: Name,
    client: NodeClient,
}
type Snapshot = Result<Arc<[Connection]>, Failure>;

/// One latest request per window; dropping the link aborts its metadata read.
#[derive(Debug)]
pub(crate) struct Hub {
    connections: watch::Sender<Snapshot>,
    active: Option<tokio::task::JoinHandle<()>>,
}
impl Default for Hub {
    fn default() -> Self {
        let (connections, _receiver) = watch::channel::<Snapshot>(Ok(Arc::from([])));
        Self {
            connections,
            active: None,
        }
    }
}
impl Drop for Hub {
    fn drop(&mut self) {
        if let Some(task) = self.active.take() {
            task.abort();
        }
    }
}
impl Hub {
    fn publish(&self, snapshot: Snapshot) {
        drop(self.connections.send_replace(snapshot));
    }
    fn request(&mut self, ask: Ask, events: mpsc::Sender<Event>) {
        if let Some(task) = self.active.take() {
            task.abort();
        }
        let receiver = self.connections.subscribe();
        self.active = Some(tokio::spawn(async move {
            let result = tokio::time::timeout(LOOKUP_DEADLINE, locate(&ask.target, receiver))
                .await
                .unwrap_or(Err(Failure::TimedOut));
            let _closed = events
                .send(Event::Sessions(Answer {
                    ticket: ask.ticket,
                    result,
                }))
                .await;
        }));
    }
}
impl State {
    /// Called after a fleet refresh and immediately when reload discards clients.
    pub(crate) fn session_connections_changed(&self) {
        let snapshot = self
            .clients
            .iter()
            .map(|(host, connected)| {
                Name::parse(host)
                    .map(|host| Connection {
                        host,
                        client: connected.client.clone(),
                    })
                    .map_err(|_invalid| Failure::InvalidHost)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|mut connections| {
                connections.sort_by(|one, two| one.host.cmp(&two.host));
                Arc::from(connections)
            });
        self.session_navigation.publish(snapshot);
    }
    /// Match `NodeId` only; a deep link never falls back to a similarly named host.
    pub(crate) fn session_navigation(&mut self, ask: Ask) {
        self.session_connections_changed();
        self.session_navigation.request(ask, self.events.clone());
    }
}
fn selected(snapshot: &Snapshot, target: &NavigationTarget) -> Result<Option<Connection>, Failure> {
    let connections = snapshot.as_ref().map_err(|failure| *failure)?;
    Ok(connections
        .iter()
        .find(|connection| match target {
            NavigationTarget::Address(address) => connection.client.node().id == address.node,
            NavigationTarget::Host { host, .. } => connection.host == *host,
        })
        .cloned())
}
async fn locate(
    target: &NavigationTarget,
    mut receiver: watch::Receiver<Snapshot>,
) -> Result<Located, Failure> {
    loop {
        // Release the watch borrow before network IO or waiting for a reconnect.
        let connection = selected(&receiver.borrow_and_update(), target)?;
        if let Some(connection) = connection {
            let session = match target {
                NavigationTarget::Address(address) => address.session,
                NavigationTarget::Host { session, .. } => *session,
            };
            match connection.client.session(session).await {
                Ok(info) if info.id == session => {
                    // A rename/replacement can arrive while the read is in flight.
                    // Bind the result to the latest authenticated name for this node.
                    let Some(current) = selected(&receiver.borrow(), target)? else {
                        continue;
                    };
                    if current.client.node().id != connection.client.node().id {
                        return Err(Failure::Offline);
                    }
                    return Ok(Located {
                        address: SessionAddress {
                            node: connection.client.node().id,
                            session,
                        },
                        card: Card::from_session(current.host.as_str(), &info),
                        host: current.host,
                    });
                }
                Ok(_) => return Err(Failure::WrongSession),
                Err(error) => {
                    if let Some(code) = error.code() {
                        return Err(Failure::Refused(code));
                    }
                    // A transport loss waits for the next authenticated connection;
                    // metadata reads are safe to reissue under the original deadline.
                }
            }
        } else if matches!(target, NavigationTarget::Host { .. }) {
            // An offline alias has no stable identity to bind a future replacement to.
            return Err(Failure::Offline);
        }
        receiver
            .changed()
            .await
            .map_err(|_closed| Failure::Offline)?;
    }
}
