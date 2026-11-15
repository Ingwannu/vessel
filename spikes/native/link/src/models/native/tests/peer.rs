//! Model-list protocol fixture; no node process, sockets or harnesses.
use super::*;
use std::{
    future::poll_fn,
    num::NonZeroUsize,
    pin::Pin,
    sync::{Arc, Mutex},
};
use vessel_client::ClientConfig;
use vessel_proto::{
    Call, ClientFrame, ClientInfo, NodeInfo, Outcome, ProtocolRange, ProtocolVersion, Reply,
    ServerFrame, Welcome,
};
use vessel_rpc::FrameTransport;

const CAPACITY: usize = MAX_READS.saturating_mul(4);
const PEER_IDLE: Duration = OUTER;
#[derive(Clone, Copy)]
pub(super) enum Respond {
    Lists,
    Empty,
    Refuse(vessel_proto::ErrorCode),
    Wait,
}
pub(super) struct Peer {
    pub(super) client: NodeClient,
    pub(super) seen: mpsc::Receiver<Method>,
    calls: Arc<Mutex<Vec<Method>>>,
    _abort: Abort,
}
async fn receive<T: FrameTransport>(io: &mut T) -> Option<ClientFrame> {
    limited(
        "model peer read",
        PEER_IDLE,
        poll_fn(|cx| Pin::new(&mut *io).poll_next(cx)),
    )
    .await
    .map(|frame| serde_json::from_str(&frame.unwrap()).unwrap())
}
async fn send<T: FrameTransport>(io: &mut T, frame: ServerFrame) {
    phase(
        "model peer readiness",
        poll_fn(|cx| Pin::new(&mut *io).poll_ready(cx)),
    )
    .await
    .unwrap();
    Pin::new(&mut *io)
        .start_send(serde_json::to_string(&frame).unwrap())
        .unwrap();
    phase(
        "model peer flush",
        poll_fn(|cx| Pin::new(&mut *io).poll_flush(cx)),
    )
    .await
    .unwrap();
}
impl Peer {
    pub(super) async fn new(protocol: ProtocolVersion, respond: Respond) -> Self {
        let (client, mut server) = vessel_rpc::memory::pair(NonZeroUsize::new(CAPACITY).unwrap());
        let node: NodeInfo = serde_json::from_value(serde_json::json!({
            "id":vessel_proto::NodeId::random(),"name":"models-fixture","hostname":"disposable",
            "os":"linux","arch":"x86_64","version":"0","protocol":ProtocolRange::CURRENT,
            "started_at":chrono::DateTime::UNIX_EPOCH,"state":"serving"
        }))
        .unwrap();
        let (notice, seen) = mpsc::channel(CAPACITY);
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&calls);
        let task = tokio::spawn(async move {
            assert!(matches!(
                receive(&mut server).await,
                Some(ClientFrame::Hello(_))
            ));
            send(
                &mut server,
                ServerFrame::Welcome(Welcome {
                    protocol,
                    node: node.clone(),
                }),
            )
            .await;
            while let Some(frame) = receive(&mut server).await {
                let ClientFrame::Request { id, call } = frame else {
                    continue;
                };
                recorded.lock().unwrap().push(call.method());
                phase("model peer observation", notice.send(call.method()))
                    .await
                    .unwrap();
                let outcome = if matches!(call, Call::NodeInfo(_)) {
                    Outcome::Ok(Reply::NodeInfo(node.clone()))
                } else {
                    match respond {
                        Respond::Wait => continue,
                        Respond::Refuse(code) => {
                            Outcome::Err(vessel_proto::WireError::new(code, "fixture diagnostic"))
                        }
                        Respond::Empty => reply(&call, false),
                        Respond::Lists => reply(&call, true),
                    }
                };
                send(&mut server, ServerFrame::Response { id, outcome }).await;
            }
        });
        let abort = Abort(task.abort_handle());
        drop(task);
        let client = phase(
            "model client handshake",
            NodeClient::over(
                client,
                ClientInfo {
                    name: "models-tests".into(),
                    version: "0".into(),
                },
                &ClientConfig::default(),
            ),
        )
        .await
        .unwrap();
        Self {
            client,
            seen,
            calls,
            _abort: abort,
        }
    }
    pub(super) fn calls(&self) -> Vec<Method> {
        self.calls.lock().unwrap().clone()
    }
    pub(super) async fn barrier(&self) {
        let _node = phase(
            "model ordered barrier",
            self.client.call(vessel_proto::GetNode {}),
        )
        .await
        .unwrap();
    }
}
fn reply(call: &Call, with_rows: bool) -> Outcome {
    if matches!(call, Call::ListModels(_)) {
        Outcome::Ok(Reply::ListModels(if with_rows {
            serde_json::from_str(r#"[{"name":"shared","provider":"fixture","api":"openai_responses","model":"fixture/large","small_model":"fixture/small"}]"#).unwrap()
        } else {
            vec![]
        }))
    } else if let Call::ListProfiles(request) = call {
        assert_eq!(request.harness, None);
        Outcome::Ok(Reply::ListProfiles(if with_rows {
            serde_json::from_str(r#"[{"name":"work","label":"Work account","harness":"codex","model":"shared"},{"name":"personal","harness":"codex","model":"shared"}]"#).unwrap()
        } else {
            vec![]
        }))
    } else if matches!(call, Call::ListAgents(_)) {
        Outcome::Ok(Reply::ListAgents(if with_rows {
            serde_json::from_str(r#"[{"agent":{"added":"custom"},"detail":{"kind":"added","added":{"label":"Custom agent","program":"fixture","source":{"from":"command"}}}}]"#).unwrap()
        } else {
            vec![]
        }))
    } else {
        panic!("unexpected model-test call: {call:?}")
    }
}
