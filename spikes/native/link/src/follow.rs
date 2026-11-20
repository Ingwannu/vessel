//! A session's record stream, and a machine this link could not reach.
//!
//! Kept out of the link root so that file stays inside its line budget.

use tokio::sync::mpsc;
use vessel_app_view::{Machine, Reach};
use vessel_client::NodeClient;
use vessel_proto::SessionId;

use crate::Event;

/// A machine whose sessions could not be listed.
pub(crate) fn unreachable(name: String, why: &str) -> Machine {
    Machine {
        name,
        reach: Reach::Unreachable(why.to_owned()),
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
    }
}

/// A session's records to the events channel, one event per record (the
/// screen folds whatever has queued up in one update).
pub(crate) async fn follow(client: NodeClient, session: SessionId, events: mpsc::Sender<Event>) {
    let lost = |why: String| Event::SessionLost { session, why };
    let mut stream = match client.follow(session, vessel_proto::EventSeq::new(0)).await {
        Ok(stream) => stream,
        Err(error) => {
            let _closed = events.send(lost(error.to_string())).await;
            return;
        }
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(record) => {
                let event = Event::Records {
                    session,
                    records: vec![record],
                };
                if events.send(event).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                let _closed = events.send(lost(error.to_string())).await;
                return;
            }
        }
    }
}
