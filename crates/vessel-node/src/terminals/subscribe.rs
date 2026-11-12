//! `terminal.subscribe`: a terminal's records to one subscription, from its
//! ring alone (a terminal keeps nothing on disk, ADR 0031).
//!
//! As a session's forwarder does: never buffers; a follower further behind
//! than the live buffer ends `lagged { resume_from }` and pages with
//! `terminal.read`; the terminal's end (its final status, then its closed
//! ring) ends it `session_ended`. A cursor older than the oldest kept
//! record starts at that record: the dropped scrollback is gone, and the
//! client sees the gap as it sees a pruned log.

use std::sync::Arc;

use tokio::sync::broadcast::error::RecvError;
use tokio_util::task::TaskTracker;
use vessel_proto::terminals::{SubscribeTerminal, TerminalSubscribed};
use vessel_proto::{EventRecord, EventSeq, SubscriptionEnd};
use vessel_rpc::{Outbound, SubscriptionSink};
use vessel_session::FollowError;
use vessel_session::terminals::TerminalHandle;

use super::registry::TerminalsError;

/// The subscription was closed by the client or its connection.
struct Closed;

/// Open a subscription to `handle` and start its forwarder. Everything that
/// can fail is checked first, so a failed call leaves nothing behind.
pub(crate) async fn open(
    handle: TerminalHandle,
    outbound: &Outbound,
    forwarders: &TaskTracker,
    request: SubscribeTerminal,
) -> Result<TerminalSubscribed, TerminalsError> {
    let first = match handle.follow(request.from) {
        Ok(first) => first,
        Err(FollowError::Ahead { next }) => {
            return Err(TerminalsError::Invalid(format!(
                "the terminal's log ends at {next}"
            )));
        }
        Err(FollowError::Gap { .. }) => {
            return Err(TerminalsError::Conflict(
                "the terminal's scrollback moved; subscribe again".to_owned(),
            ));
        }
    };
    let terminal = handle.info();
    // Where the records start: the oldest kept one when `from` is older.
    let start = first
        .cached
        .first()
        .map_or(request.from.max(terminal.oldest_seq), |record| record.seq);
    let sink = outbound
        .open_subscription(start)
        .await
        .map_err(|_closed| TerminalsError::Conflict("the connection is closing".to_owned()))?;
    let subscription = sink.id();
    let forwarder = Forwarder {
        cursor: start,
        sink,
    };
    drop(forwarders.spawn(forwarder.run(first)));
    Ok(TerminalSubscribed {
        subscription,
        terminal,
    })
}

struct Forwarder {
    cursor: EventSeq,
    sink: SubscriptionSink,
}

impl Forwarder {
    async fn run(mut self, follow: vessel_session::Follow) {
        if let Ok(end) = self.stream(follow).await {
            self.sink.end(end).await;
        } else {
            tracing::trace!("terminal subscription closed by the client");
        }
    }

    async fn stream(&mut self, follow: vessel_session::Follow) -> Result<SubscriptionEnd, Closed> {
        let vessel_session::Follow { cached, mut live } = follow;
        for record in cached {
            self.send(record).await?;
        }
        loop {
            let received = tokio::select! {
                biased;
                () = self.sink.cancelled() => return Err(Closed),
                received = live.recv() => received,
            };
            match received {
                Ok(record) => self.send(record).await?,
                Err(RecvError::Lagged(_missed)) => {
                    return Ok(SubscriptionEnd::Lagged {
                        resume_from: self.cursor,
                    });
                }
                Err(RecvError::Closed) => return Ok(SubscriptionEnd::SessionEnded),
            }
        }
    }

    async fn send(&mut self, record: Arc<EventRecord>) -> Result<(), Closed> {
        let seq = record.seq;
        self.sink.send(record).await.map_err(|_closed| Closed)?;
        self.cursor = seq.next().unwrap_or(seq);
        Ok(())
    }
}
