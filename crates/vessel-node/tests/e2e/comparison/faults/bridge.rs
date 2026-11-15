//! Bounded test bridge; no product hooks, altered RPC guards or synthetic replies.
use crate::support::{Fixture, client, step, url};
use futures_util::{SinkExt as _, StreamExt as _};
use std::num::NonZeroUsize;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest as _, http::HeaderValue};
use vessel_client::NodeClient;
use vessel_proto::{Call, ClientFrame, Method, RequestId, ServerFrame};

enum Command {
    Hold {
        method: Method,
        after: usize,
        arrival: oneshot::Sender<()>,
        ack: oneshot::Sender<()>,
    },
    Release,
    ReleaseRenamed(vessel_proto::AttachmentName),
    Count {
        method: Method,
        reply: oneshot::Sender<usize>,
    },
    Stop,
}
pub(crate) struct Bridge {
    pub(crate) client: NodeClient,
    command: mpsc::Sender<Command>,
    worker: tokio::task::JoinHandle<()>,
}
impl Bridge {
    pub(crate) async fn open(fixture: &Fixture, node: &vessel_node::Node) -> Self {
        Self::open_with(fixture, node, None).await
    }
    pub(crate) async fn at_protocol(
        fixture: &Fixture,
        node: &vessel_node::Node,
        protocol: vessel_proto::ProtocolVersion,
    ) -> Self {
        Self::open_with(fixture, node, Some(protocol)).await
    }
    async fn open_with(
        fixture: &Fixture,
        node: &vessel_node::Node,
        protocol: Option<vessel_proto::ProtocolVersion>,
    ) -> Self {
        let mut request = url(node).to_string().into_client_request().unwrap();
        let _old = request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", fixture.token().expose())).unwrap(),
        );
        let (mut websocket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let (local, mut remote) = vessel_rpc::memory::pair(NonZeroUsize::new(16).unwrap());
        let (command, mut commands) = mpsc::channel(1);
        let worker = tokio::spawn(async move {
            let mut calls: Vec<(RequestId, Method)> = Vec::new();
            let mut counts: Vec<(Method, usize)> = Vec::new();
            let mut rule: Option<(Method, usize, oneshot::Sender<()>)> = None;
            let mut held: Option<String> = None;
            loop {
                tokio::select! {
                    frame = remote.next() => {
                        let Some(Ok(mut text)) = frame else { break; };
                        let mut decoded: ClientFrame = serde_json::from_str(&text).unwrap();
                        if let ClientFrame::Hello(hello) = &mut decoded && let Some(protocol) = protocol {
                            hello.protocol.max = protocol;
                            text = serde_json::to_string(&decoded).unwrap();
                        }
                        if let ClientFrame::Request { id, call } = decoded {
                            let method = if let Call::Forward(forward) = &call { forward.call.method() } else { call.method() };
                            assert!(calls.len() < 64);
                            calls.push((id, method));
                            if let Some((_, count)) = counts.iter_mut().find(|(seen, _)| *seen == method) { *count = count.saturating_add(1); }
                            else { counts.push((method, 1)); }
                        }
                        if websocket.send(Message::Text(text.into())).await.is_err() { break; }
                    }
                    frame = websocket.next() => {
                        let text = match frame {
                            Some(Ok(Message::Text(text))) => text,
                            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => continue,
                            Some(Ok(Message::Binary(_) | Message::Close(_)) | Err(_)) | None => break,
                        };
                        let decoded: ServerFrame = serde_json::from_str(text.as_str()).unwrap();
                        let method = if let ServerFrame::Response { id, .. } = decoded {
                            calls.iter().position(|(pending, _)| *pending == id).map(|at| calls.swap_remove(at).1)
                        } else { None };
                        let intercept = if let Some((selected, after, _)) = &mut rule {
                            if method == Some(*selected) {
                                if *after == 0 { true } else { *after = after.saturating_sub(1); false }
                            } else { false }
                        } else { false };
                        if intercept {
                            let (_, _, arrival) = rule.take().unwrap();
                            held = Some(text.to_string());
                            let _delivered = arrival.send(());
                        } else if remote.send(text.to_string()).await.is_err() { break; }
                    }
                    command = commands.recv() => match command {
                        Some(Command::Hold { method, after, arrival, ack }) => {
                            assert!(rule.is_none() && held.is_none());
                            rule = Some((method, after, arrival));
                            ack.send(()).unwrap();
                        }
                        Some(Command::Release) => if let Some(text) = held.take() { remote.send(text).await.unwrap(); },
                        Some(Command::ReleaseRenamed(name)) => if let Some(text) = held.take() {
                            remote.send(renamed_finish(&text, name)).await.unwrap();
                        },
                        Some(Command::Count { method, reply }) => {
                            reply.send(counts.iter().find(|(seen, _)| *seen == method).map_or(0, |(_, count)| *count)).unwrap();
                        }
                        Some(Command::Stop) | None => break,
                    }
                }
            }
        });
        let client = NodeClient::over(local, client(), &vessel_client::ClientConfig::default())
            .await
            .unwrap();
        Self {
            client,
            command,
            worker,
        }
    }
    pub(crate) async fn hold(&self, method: Method) -> oneshot::Receiver<()> {
        self.hold_after(method, 0).await
    }
    pub(crate) async fn hold_after(&self, method: Method, after: usize) -> oneshot::Receiver<()> {
        let (arrival, arrived) = oneshot::channel();
        let (ack, armed) = oneshot::channel();
        self.command
            .send(Command::Hold {
                method,
                after,
                arrival,
                ack,
            })
            .await
            .unwrap();
        step("arm response gate", armed).await.unwrap();
        arrived
    }
    pub(crate) async fn release(&self) {
        self.command.send(Command::Release).await.unwrap();
    }
    pub(crate) async fn release_renamed(&self, name: vessel_proto::AttachmentName) {
        self.command
            .send(Command::ReleaseRenamed(name))
            .await
            .unwrap();
    }
    pub(crate) async fn count(&self, method: Method) -> usize {
        let (reply, answer) = oneshot::channel();
        self.command
            .send(Command::Count { method, reply })
            .await
            .unwrap();
        step("count actual RPC requests", answer).await.unwrap()
    }
    pub(crate) async fn stop(self) {
        self.command.send(Command::Stop).await.unwrap();
        self.worker.await.unwrap();
    }
}

// Alter the acknowledgement only after a real finish has stored its file.
// The original id is preserved so the test can inspect the actual effect.
fn renamed_finish(text: &str, name: vessel_proto::AttachmentName) -> String {
    let mut frame: ServerFrame = serde_json::from_str(text).unwrap();
    let ServerFrame::Response {
        outcome: vessel_proto::Outcome::Ok(vessel_proto::Reply::Forward(forwarded)),
        ..
    } = &mut frame
    else {
        panic!("a forwarded finish response");
    };
    let vessel_proto::Reply::FinishUpload(file) = forwarded.reply.as_mut() else {
        panic!("a finished attachment");
    };
    file.name = name;
    serde_json::to_string(&frame).unwrap()
}
