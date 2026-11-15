//! One browser tab, after its hello: calls, streams and ops in, replies,
//! records and node states out.
//!
//! [Decision Log]
//! - Purpose: route the browser's frames to the nodes' shared connections
//!   ([`Registry`]) and back, without interpreting the protocol.
//! - Calls are decoded into the protocol's typed `Call`, checked and
//!   forwarded over the node's shared connection (`calls`).
//! - Streams follow sessions over the node's shared connection, gap-free
//!   across drops (`streams`).
//! - Ops are what the tab asks the bridge itself (`ops`).
//! - Limits per tab: [`MAX_CALLS`] calls and ops at once and
//!   `streams::MAX_FOLLOWS` streams. Frames out go through one bounded queue
//!   ([`OUTBOX`]); a tab that reads slowly slows its own streams, nobody
//!   else's.
//! - Everything a tab started stops when its socket closes.
//!
//! [Decision Log] The page's preferences and drafts (ADR 0028)
//! - Purpose: two changes from one tab must apply, and be answered, in
//!   the order the tab sent them; a draft saved as the page is hidden must
//!   not be refused because node calls hold every permit.
//! - Choice: those ops (`ops::route`) are handed, from the read loop and
//!   in arrival order, to the task that keeps their file (`keep`), without
//!   a permit; its full queue answers `busy` at once. Each answer comes
//!   back on a oneshot, and one writer per tab ([`kept_replies`]) awaits
//!   them in the order they were asked, so replies leave in that order
//!   too. The preferences' published states reach the tab through a
//!   `watch` ([`preferences`]): the current one after the welcome, then
//!   each new one, the latest only.
//! - Alternatives: spawning each as one of the tab's tasks, as the other
//!   ops are (two changes could be applied out of order, and an older
//!   reply could overtake a newer push); answering inside the read loop
//!   (a slow disk would stop the tab reading).

use std::collections::HashSet;
use std::sync::Arc;

use crate::socket::WebSocket;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use tokio::sync::{Semaphore, broadcast, mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::Message;
use vessel_proto::{NodeId, RawJson};

use crate::calls;
use crate::keep::{Answer, Keeper};
use crate::ops::{BridgeOp, Routed};
use crate::registry::{Cancelled, Change, Registry};
use crate::source::NodeSource;
use crate::streams::{Followed, Follows, Place};
use crate::wire::{
    CallId, CallOutcome, ClientFrame, Failure, NodeKey, Op, OpFailure, OpOutcome, PreferencesState,
    ServerFrame, StreamId, StreamState, StreamStop,
};

/// Calls and ops one tab may have in flight.
const MAX_CALLS: usize = 64;

/// Frames waiting to be written to one tab.
const OUTBOX: usize = 256;

/// Answers of the preferences and drafts ops one tab waits for, in the
/// order asked; past it the tab's read loop waits (its own frames only).
const KEPT_REPLIES: usize = 128;

/// An answer the tab waits for from `keep`, by the op's id.
type Pending = (CallId, oneshot::Receiver<Answer>);

/// Serve one tab until its socket closes.
pub(crate) async fn run<S: NodeSource>(
    registry: Arc<Registry<S>>,
    keeper: Keeper,
    socket: WebSocket,
) {
    let (sink, stream) = socket.split();
    let (out, outbox) = mpsc::channel(OUTBOX);
    let (pending, waiting) = mpsc::channel(KEPT_REPLIES);
    tokio::select! {
        () = write(sink, outbox) => {}
        () = statuses(&registry, out.clone()) => {}
        () = preferences(keeper.states(), out.clone()) => {}
        () = kept_replies(waiting, out.clone()) => {}
        () = read(&registry, keeper, pending, stream, out) => {}
    }
}

/// The preferences now, then every state published after it. Only the
/// UI stopping ends the `watch`; the tab goes on without pushes then.
async fn preferences(
    mut states: watch::Receiver<Arc<PreferencesState>>,
    out: mpsc::Sender<ServerFrame>,
) {
    loop {
        let preferences = Arc::clone(&states.borrow_and_update());
        if out
            .send(ServerFrame::Preferences { preferences })
            .await
            .is_err()
        {
            return;
        }
        if states.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// Write the answers from `keep` in the order they were asked.
async fn kept_replies(mut waiting: mpsc::Receiver<Pending>, out: mpsc::Sender<ServerFrame>) {
    while let Some((id, answer)) = waiting.recv().await {
        let outcome = match answer.await {
            Ok(Ok(result)) => OpOutcome::Ok(result),
            Ok(Err(error)) => OpOutcome::Err {
                reason: error.reason,
                message: error.message,
            },
            Err(_dropped) => OpOutcome::Err {
                reason: OpFailure::Failed,
                message: Cancelled::Stopping.message().to_owned(),
            },
        };
        if out
            .send(ServerFrame::OpReply { id, outcome })
            .await
            .is_err()
        {
            return;
        }
    }
}

/// Write frames to the browser until it goes.
async fn write(mut sink: SplitSink<WebSocket, Message>, mut outbox: mpsc::Receiver<ServerFrame>) {
    while let Some(frame) = outbox.recv().await {
        let text = match serde_json::to_string(&frame) {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(%error, "could not encode a frame for the browser");
                continue;
            }
        };
        if sink.send(Message::Text(text.into())).await.is_err() {
            return;
        }
    }
}

/// Every node's state now, then each change.
async fn statuses<S: NodeSource>(registry: &Registry<S>, out: mpsc::Sender<ServerFrame>) {
    let mut changes = registry.changes();
    let mut everything = true;
    let mut known = HashSet::new();
    loop {
        if everything {
            // A lagged receiver may have missed a removal. Reconcile its
            // known set against the snapshot instead of retaining stale Up.
            let snapshot = registry.statuses();
            let current: HashSet<_> = snapshot.iter().map(|(node, _)| node.clone()).collect();
            for node in known.difference(&current) {
                if out.send(removed(node.clone())).await.is_err() {
                    return;
                }
            }
            known = current;
            for (node, status) in snapshot {
                if out.send(ServerFrame::Node { node, status }).await.is_err() {
                    return;
                }
            }
            everything = false;
        }
        let frame = match changes.recv().await {
            Ok(Change::State { node, status }) => {
                let _new = known.insert(node.clone());
                ServerFrame::Node {
                    node,
                    status: *status,
                }
            }
            Ok(Change::Removed { node }) => {
                let _gone = known.remove(&node);
                removed(node)
            }
            Err(broadcast::error::RecvError::Lagged(_missed)) => {
                // Discard already superseded queued changes before the
                // snapshot, keeping the per-tab key set bounded.
                changes = registry.changes();
                everything = true;
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => return,
        };
        if out.send(frame).await.is_err() {
            return;
        }
    }
}

fn removed(node: NodeKey) -> ServerFrame {
    ServerFrame::Node {
        node,
        status: crate::wire::NodeStatus::Failed {
            message: Cancelled::Removed.message().to_owned(),
        },
    }
}

/// Read the browser's frames, starting what they ask for, until it goes.
/// Dropping the tab's tasks on return stops everything it started.
async fn read<S: NodeSource>(
    registry: &Arc<Registry<S>>,
    keeper: Keeper,
    pending: mpsc::Sender<Pending>,
    mut stream: SplitStream<WebSocket>,
    out: mpsc::Sender<ServerFrame>,
) {
    let mut tab = Tab {
        registry: Arc::clone(registry),
        keeper,
        pending,
        out,
        tasks: JoinSet::new(),
        follows: Follows::default(),
        calls: Arc::new(Semaphore::new(MAX_CALLS)),
    };
    loop {
        let message = tokio::select! {
            message = stream.next() => message,
            Some(_finished) = tab.tasks.join_next(), if !tab.tasks.is_empty() => continue,
        };
        let text = match message {
            Some(Ok(Message::Text(text))) => text,
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(Message::Binary(_) | Message::Frame(_))) => {
                unreadable(&tab.out, "frames are JSON text".to_owned()).await;
                continue;
            }
            Some(Ok(Message::Close(_)) | Err(_)) | None => return,
        };
        match serde_json::from_str::<ClientFrame>(&text) {
            Ok(frame) => tab.handle(frame).await,
            Err(error) => refuse(&tab.out, &text, &error).await,
        }
    }
}

/// What a frame that did not decode still says: its kind and the id to
/// answer by.
///
/// Names in a frame are parsed as it is decoded (`wire::NodeKey`,
/// `wire::BookName`), so a call, op or follow naming a node badly fails
/// the whole frame. The page waits for an answer by id; this reads just
/// enough to give one, typed, instead of an `unreadable` it cannot match
/// to what it sent. Anything else is unreadable.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Addressed {
    Call {
        id: CallId,
    },
    Op {
        id: CallId,
    },
    Follow {
        stream: StreamId,
    },
    #[serde(other)]
    Other,
}

/// Answer a frame that did not decode, by its id when it has one.
async fn refuse(out: &mpsc::Sender<ServerFrame>, text: &str, error: &serde_json::Error) {
    let message = format!("not a frame this bridge reads: {error}");
    let frame = match serde_json::from_str::<Addressed>(text) {
        Ok(Addressed::Call { id }) => ServerFrame::Reply {
            id,
            outcome: CallOutcome::Err(Failure::Refused { message }),
        },
        Ok(Addressed::Op { id }) => ServerFrame::OpReply {
            id,
            outcome: OpOutcome::Err {
                reason: OpFailure::Unreadable,
                message,
            },
        },
        Ok(Addressed::Follow { stream }) => ServerFrame::Stream {
            stream,
            state: StreamState::Failed {
                message,
                reason: StreamStop::Error,
            },
        },
        Ok(Addressed::Other) | Err(_) => ServerFrame::Unreadable { message },
    };
    let _gone = out.send(frame).await;
}

/// What one tab started.
struct Tab<S> {
    registry: Arc<Registry<S>>,
    /// The page's preferences and drafts.
    keeper: Keeper,
    /// Answers from `keep` this tab waits for, in order.
    pending: mpsc::Sender<Pending>,
    out: mpsc::Sender<ServerFrame>,
    tasks: JoinSet<()>,
    follows: Follows,
    /// Permits for calls and ops in flight.
    calls: Arc<Semaphore>,
}

impl<S: NodeSource> Tab<S> {
    async fn handle(&mut self, frame: ClientFrame) {
        match frame {
            ClientFrame::Hello { .. } => {
                unreadable(&self.out, "this socket is already welcomed".to_owned()).await;
            }
            ClientFrame::Call {
                id,
                node,
                call,
                expected_node,
            } => self.call(id, node, call, expected_node).await,
            ClientFrame::Follow {
                stream,
                node,
                session,
                from,
            } => {
                let place = Place {
                    tasks: &mut self.tasks,
                    registry: &self.registry,
                    out: &self.out,
                };
                self.follows
                    .start(place, stream, node, Followed::Session(session), from)
                    .await;
            }
            ClientFrame::FollowTerminal {
                stream,
                node,
                terminal,
                from,
            } => {
                let place = Place {
                    tasks: &mut self.tasks,
                    registry: &self.registry,
                    out: &self.out,
                };
                self.follows
                    .start(place, stream, node, Followed::Terminal(terminal), from)
                    .await;
            }
            ClientFrame::Unfollow { stream } => self.follows.stop(stream),
            ClientFrame::Op { id, op } => self.op(id, op).await,
        }
    }

    async fn call(
        &mut self,
        id: CallId,
        node: NodeKey,
        call: RawJson,
        expected_node: Option<NodeId>,
    ) {
        let Ok(permit) = Arc::clone(&self.calls).try_acquire_owned() else {
            let outcome = CallOutcome::Err(calls::busy());
            let _gone = self.out.send(ServerFrame::Reply { id, outcome }).await;
            return;
        };
        let registry = Arc::clone(&self.registry);
        let out = self.out.clone();
        let _call = self.tasks.spawn(async move {
            let outcome = calls::forward(&registry, &node, call, expected_node).await;
            let _gone = out.send(ServerFrame::Reply { id, outcome }).await;
            drop(permit);
        });
    }

    async fn op(&mut self, id: CallId, op: Op) {
        match crate::ops::route(op) {
            Routed::Bridge(op) => self.bridge_op(id, op).await,
            Routed::Kept(op) => {
                // Handed over now, in arrival order; awaited by
                // `kept_replies`, so this loop never waits on the disk.
                let answer = self.keeper.ask(op);
                let _gone = self.pending.send((id, answer)).await;
            }
        }
    }

    async fn bridge_op(&mut self, id: CallId, op: BridgeOp) {
        let Ok(permit) = Arc::clone(&self.calls).try_acquire_owned() else {
            let error = crate::ops::OpError::busy();
            let outcome = OpOutcome::Err {
                reason: error.reason,
                message: error.message,
            };
            let _gone = self.out.send(ServerFrame::OpReply { id, outcome }).await;
            return;
        };
        let registry = Arc::clone(&self.registry);
        let out = self.out.clone();
        let _op = self.tasks.spawn(async move {
            let outcome = match crate::ops::answer(&registry, op).await {
                Ok(result) => OpOutcome::Ok(result),
                Err(error) => OpOutcome::Err {
                    reason: error.reason,
                    message: error.message,
                },
            };
            let _gone = out.send(ServerFrame::OpReply { id, outcome }).await;
            drop(permit);
        });
    }
}

async fn unreadable(out: &mpsc::Sender<ServerFrame>, message: String) {
    let _gone = out.send(ServerFrame::Unreadable { message }).await;
}
