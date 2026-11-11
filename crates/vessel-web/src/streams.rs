//! One tab's followed sessions: the table of its streams, each stream's
//! task (`follow_resuming` over the node's shared connection, so it
//! survives the connection dropping, gap-free) and how a stream ends.
//!
//! A stream ends when the session does, when it fails for good, when the
//! tab stops it, or when its node's entry is cancelled (the node left the
//! book, or the UI is stopping: `registry::Cancelled`). Its tasks belong
//! to the tab's `JoinSet`, so they stop when the tab's socket closes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinSet};
use vessel_client::{StreamError, follow_resuming};
use vessel_proto::{EventSeq, SessionId};

use crate::calls::failure_text;
use crate::registry::{Registry, Shared, chain};
use crate::source::NodeSource;
use crate::wire::{NodeKey, ServerFrame, StreamId, StreamState, StreamStop};

/// Sessions one tab may follow at once.
pub(crate) const MAX_FOLLOWS: usize = 64;

/// Where a tab's stream runs: the tab's tasks, the nodes, and its outbox.
pub(crate) struct Place<'tab, S> {
    pub(crate) tasks: &'tab mut JoinSet<()>,
    pub(crate) registry: &'tab Arc<Registry<S>>,
    pub(crate) out: &'tab mpsc::Sender<ServerFrame>,
}

/// One tab's streams, by the tab's id for each.
#[derive(Debug, Default)]
pub(crate) struct Follows {
    table: HashMap<StreamId, AbortHandle>,
}

impl Follows {
    /// Follow `session` on `node` from `from` as `stream`, or say why not.
    pub(crate) async fn start<S: NodeSource>(
        &mut self,
        place: Place<'_, S>,
        stream: StreamId,
        node: NodeKey,
        session: SessionId,
        from: EventSeq,
    ) {
        self.table.retain(|_stream, task| !task.is_finished());
        let refusal = if self.table.contains_key(&stream) {
            Some("this stream id is in use")
        } else if self.table.len() >= MAX_FOLLOWS {
            Some("too many sessions followed at once")
        } else {
            None
        };
        if let Some(message) = refusal {
            let state = StreamState::Failed {
                message: message.to_owned(),
                reason: StreamStop::Error,
            };
            let _gone = place.out.send(ServerFrame::Stream { stream, state }).await;
            return;
        }
        let task = place.tasks.spawn(follow(
            Arc::clone(place.registry),
            place.out.clone(),
            stream,
            node,
            session,
            from,
        ));
        let _previous = self.table.insert(stream, task);
    }

    /// Stop following `stream`; no more records for it.
    pub(crate) fn stop(&mut self, stream: StreamId) {
        if let Some(task) = self.table.remove(&stream) {
            task.abort();
        }
    }
}

/// Follow `session` on `node` from `from`, forwarding until it ends, it
/// fails, or the tab stops it.
async fn follow<S: NodeSource>(
    registry: Arc<Registry<S>>,
    out: mpsc::Sender<ServerFrame>,
    stream: StreamId,
    node: NodeKey,
    session: SessionId,
    from: EventSeq,
) {
    let entry = match registry.ensure(&node).await {
        Ok(entry) => entry,
        Err(failure) => {
            let state = StreamState::Failed {
                message: failure_text(&failure),
                reason: StreamStop::Error,
            };
            let _gone = out.send(ServerFrame::Stream { stream, state }).await;
            return;
        }
    };
    let client = registry.client_info().clone();
    let removed = entry.stop.clone();
    let mut records = follow_resuming(Shared::new(entry), client, session, from);
    loop {
        let (frame, last) = match tokio::select! {
            record = records.next() => record,
            () = removed.cancelled() => {
                drop(records);
                let state = cut_off(&registry);
                let _gone = out.send(ServerFrame::Stream { stream, state }).await;
                return;
            }
        } {
            None => (
                ServerFrame::Stream {
                    stream,
                    state: StreamState::Ended,
                },
                true,
            ),
            Some(Ok(record)) => (ServerFrame::Event { stream, record }, false),
            // The stream can see the closed connection before this task
            // sees the cancellation: say why it was cut off, not how.
            Some(Err(_error)) if removed.is_cancelled() => (
                ServerFrame::Stream {
                    stream,
                    state: cut_off(&registry),
                },
                true,
            ),
            Some(Err(error)) => {
                let (state, last) = stream_state(error);
                (ServerFrame::Stream { stream, state }, last)
            }
        };
        if out.send(frame).await.is_err() || last {
            return;
        }
    }
}

/// A stream's last notice when its node's entry was cancelled.
fn cut_off<S: NodeSource>(registry: &Registry<S>) -> StreamState {
    let cancelled = registry.cancelled();
    StreamState::Failed {
        message: cancelled.message().to_owned(),
        reason: cancelled.reason(),
    }
}

/// A stream notice as the browser sees it, and whether it is the last.
fn stream_state(error: StreamError) -> (StreamState, bool) {
    match error {
        StreamError::Missing {
            expected,
            resumed_at,
        } => (
            StreamState::Missing {
                expected,
                resumed_at,
            },
            false,
        ),
        StreamError::Reconnecting { reason, .. } => {
            (StreamState::Reconnecting { message: reason }, false)
        }
        StreamError::Resumed { at } => (StreamState::Resumed { at }, false),
        StreamError::Call(_) | StreamError::Unsubscribed | StreamError::Unreachable { .. } => (
            StreamState::Failed {
                message: chain(&error),
                reason: StreamStop::Error,
            },
            true,
        ),
    }
}
