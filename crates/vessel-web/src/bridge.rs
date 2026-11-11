//! One browser tab, after its hello: calls, streams and ops in, replies,
//! records and node states out.
//!
//! [Decision Log]
//! - Purpose: route the browser's frames to the nodes' shared connections
//!   ([`Registry`]) and back, without interpreting the protocol.
//! - Calls: the browser's `Call` is decoded into the typed `Call` (never an
//!   untyped tree), checked against [`forwardable`] and sent with
//!   `NodeClient::call_any`, whose protocol-version gate is the typed
//!   calls'. What a browser may do on a node is the node's to decide, as
//!   for the CLI. A call in flight when the connection drops fails as
//!   `disconnected` and is never repeated (a `session.send` must not run
//!   twice).
//! - Streams: `follow_resuming` over the node's shared connection, so a
//!   stream survives the node's connection dropping, gap-free (`streams`).
//! - Limits per tab: [`MAX_CALLS`] calls and ops at once and
//!   `streams::MAX_FOLLOWS` streams. Frames out go through one bounded queue
//!   ([`OUTBOX`]); a tab that reads slowly slows its own streams, nobody
//!   else's.
//! - Everything a tab started stops when its socket closes.
//! - The ops (what the tab asks the bridge itself) are `ops`'s.

use std::collections::HashSet;
use std::sync::Arc;

use crate::socket::WebSocket;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::Deserialize;
use tokio::sync::{Semaphore, broadcast, mpsc};
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::Message;
use vessel_client::{CallError, UnaryCall};
use vessel_proto::{Call, Method, RawJson};

use crate::registry::{Cancelled, Change, Registry};
use crate::source::NodeSource;
use crate::streams::{Follows, Place};
use crate::wire::{
    CallId, CallOutcome, ClientFrame, Failure, NodeKey, Op, OpFailure, OpOutcome, ServerFrame,
    StreamId, StreamState, StreamStop,
};

/// Calls and ops one tab may have in flight.
const MAX_CALLS: usize = 64;

/// Frames waiting to be written to one tab.
const OUTBOX: usize = 256;

/// Serve one tab until its socket closes.
pub(crate) async fn run<S: NodeSource>(registry: Arc<Registry<S>>, socket: WebSocket) {
    let (sink, stream) = socket.split();
    let (out, outbox) = mpsc::channel(OUTBOX);
    tokio::select! {
        () = write(sink, outbox) => {}
        () = statuses(&registry, out.clone()) => {}
        () = read(&registry, stream, out) => {}
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
                ServerFrame::Node { node, status }
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
    mut stream: SplitStream<WebSocket>,
    out: mpsc::Sender<ServerFrame>,
) {
    let mut tab = Tab {
        registry: Arc::clone(registry),
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
            ClientFrame::Call { id, node, call } => self.call(id, node, call).await,
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
                self.follows.start(place, stream, node, session, from).await;
            }
            ClientFrame::Unfollow { stream } => self.follows.stop(stream),
            ClientFrame::Op { id, op } => self.op(id, op).await,
        }
    }

    async fn call(&mut self, id: CallId, node: NodeKey, call: RawJson) {
        let Ok(permit) = Arc::clone(&self.calls).try_acquire_owned() else {
            let outcome = CallOutcome::Err(busy());
            let _gone = self.out.send(ServerFrame::Reply { id, outcome }).await;
            return;
        };
        let registry = Arc::clone(&self.registry);
        let out = self.out.clone();
        let _call = self.tasks.spawn(async move {
            let outcome = forward(&registry, &node, call).await;
            let _gone = out.send(ServerFrame::Reply { id, outcome }).await;
            drop(permit);
        });
    }

    async fn op(&mut self, id: CallId, op: Op) {
        let Ok(permit) = Arc::clone(&self.calls).try_acquire_owned() else {
            let outcome = OpOutcome::Err {
                reason: OpFailure::Busy,
                message: "too many requests at once".to_owned(),
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

fn busy() -> Failure {
    Failure::Refused {
        message: "too many calls at once".to_owned(),
    }
}

/// Decode `call`, pass it to `node`, and say how it went.
async fn forward<S: NodeSource>(
    registry: &Arc<Registry<S>>,
    node: &NodeKey,
    call: RawJson,
) -> CallOutcome {
    let call = match decode(&call) {
        Ok(call) => call,
        Err(failure) => return CallOutcome::Err(failure),
    };
    let client = match registry.client(node).await {
        Ok(client) => client,
        Err(failure) => return CallOutcome::Err(failure),
    };
    match client.call_any(call).await {
        Ok(reply) => CallOutcome::Ok(Box::new(reply)),
        Err(error) => CallOutcome::Err(failure(error)),
    }
}

/// The browser's call, typed and checked.
fn decode(call: &RawJson) -> Result<UnaryCall, Failure> {
    let call: Call = serde_json::from_str(call.as_text()).map_err(|error| Failure::Refused {
        message: format!("not a call this vessel knows: {error}"),
    })?;
    let method = call.method();
    if !forwardable(method) {
        return Err(Failure::Refused {
            message: format!("`{}` is not for browsers", method.as_str()),
        });
    }
    UnaryCall::try_from(call).map_err(|error| Failure::Refused {
        message: error.to_string(),
    })
}

/// Whether a browser may call `method` through the bridge (ADR 0017: one
/// line per method, decided here when the method is added).
///
/// Refused: the connection-scoped subscription methods (streams have their
/// own frames), and the fleet's node-to-node machinery, which only nodes
/// and relays call. Everything else passes, and the node checks scopes as
/// for any client.
const fn forwardable(method: Method) -> bool {
    match method {
        Method::NodeInfo
        | Method::ListHarnesses
        | Method::ListSessions
        | Method::GetSession
        | Method::CreateSession
        | Method::SendInput
        | Method::KillSession
        | Method::ReadEvents
        | Method::StopNode
        | Method::InviteDevice
        | Method::ListDevices
        | Method::RevokeDevice
        | Method::ListProfiles
        | Method::ReloadProfiles
        | Method::MoveProfile
        | Method::GetManifest
        | Method::ReadBlob
        | Method::WatchFleet
        | Method::ListMembers
        | Method::Forward
        | Method::ListNodes
        | Method::ForgetNode
        | Method::ExportSession
        | Method::ImportSession
        | Method::ExportMemory
        | Method::ImportMemory
        | Method::ReadTransfer
        | Method::CloseTransfer
        | Method::SetSecret
        | Method::ListSecrets
        | Method::DeleteSecret
        | Method::CopySecret
        | Method::LspWorkspaceSymbols
        | Method::LspDocumentSymbols
        | Method::LspDefinition
        | Method::LspReferences
        | Method::LspHover
        | Method::LspImplementations
        | Method::LspTypeDefinition
        | Method::LspIncomingCalls
        | Method::LspOutgoingCalls
        | Method::LspDiagnostics
        | Method::ListModels
        | Method::HarnessOptions
        | Method::SessionOptions
        | Method::SetSessionOption
        | Method::RenameSession => true,
        Method::Subscribe
        | Method::Unsubscribe
        | Method::ReportApplied
        | Method::Relay
        | Method::Deliver
        | Method::GrantHome
        | Method::GetCallback
        | Method::ListPeers => false,
    }
}

fn failure(error: CallError) -> Failure {
    match error {
        CallError::Remote(error) => Failure::Remote { error },
        CallError::NotInProtocol {
            method,
            needs,
            negotiated,
        } => Failure::NotInProtocol {
            method,
            needs,
            negotiated,
        },
        CallError::Disconnected => Failure::Disconnected,
        CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted => Failure::Failed {
            message: error.to_string(),
        },
    }
}

/// A failure as one line, for a stream notice or an op's answer.
pub(crate) fn failure_text(failure: &Failure) -> String {
    match failure {
        Failure::Remote { error } => error.to_string(),
        Failure::NotInProtocol { method, .. } => {
            format!("`{}` is newer than the node", method.as_str())
        }
        Failure::Disconnected => "the connection closed".to_owned(),
        Failure::Unreachable { message }
        | Failure::Refused { message }
        | Failure::Failed { message } => message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriptions_and_node_machinery_are_not_forwarded() {
        let refused: Vec<_> = Method::ALL
            .iter()
            .filter(|method| !forwardable(**method))
            .map(|method| method.as_str())
            .collect();
        assert_eq!(
            refused,
            [
                "session.subscribe",
                "session.unsubscribe",
                "fleet.report",
                "fleet.relay",
                "fleet.deliver",
                "fleet.grant",
                "fleet.callback",
                "fleet.peers",
            ]
        );
    }

    #[test]
    fn a_call_is_decoded_typed_or_refused() {
        let call = RawJson::from_text(r#"{"method":"session.list","params":null}"#).unwrap();
        assert_eq!(decode(&call).unwrap().method(), Method::ListSessions);
        for refused in [
            r#"{"method":"session.subscribe","params":{"session":"00000000-0000-0000-0000-000000000000","from":0}}"#,
            r#"{"method":"no.such","params":null}"#,
            r#"{"method":"session.get","params":{"nope":1}}"#,
            "[1,2]",
        ] {
            let call = RawJson::from_text(refused).unwrap();
            assert!(
                matches!(decode(&call), Err(Failure::Refused { .. })),
                "{refused}"
            );
        }
    }
}
