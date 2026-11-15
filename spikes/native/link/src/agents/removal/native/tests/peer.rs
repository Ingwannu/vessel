//! Existing RPC memory transport, with real client handshake/cancellation.
use std::future::poll_fn;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, ErrorCode, GetNode, NodeId, NodeInfo, NodeState, Outcome,
    ProtocolRange, ProtocolVersion, Reply, ServerFrame, Welcome, WireError,
};
use vessel_rpc::FrameTransport;

const CAPACITY: usize = 32;

pub(super) enum Response {
    Ack,
    Error(ErrorCode),
    Disconnect,
    WrongMethod,
    Wait,
}

pub(super) struct Peer {
    pub(super) client: NodeClient,
    pub(super) seen: mpsc::Receiver<Call>,
    calls: Arc<Mutex<Vec<Call>>>,
    task: tokio::task::JoinHandle<()>,
}

async fn read<T: FrameTransport>(transport: &mut T) -> Option<ClientFrame> {
    poll_fn(|cx| Pin::new(&mut *transport).poll_next(cx))
        .await
        .map(|frame| serde_json::from_str(&frame.unwrap()).unwrap())
}

async fn write<T: FrameTransport>(transport: &mut T, frame: ServerFrame) {
    poll_fn(|cx| Pin::new(&mut *transport).poll_ready(cx))
        .await
        .unwrap();
    Pin::new(&mut *transport)
        .start_send(serde_json::to_string(&frame).unwrap())
        .unwrap();
    poll_fn(|cx| Pin::new(&mut *transport).poll_flush(cx))
        .await
        .unwrap();
}

impl Peer {
    pub(super) async fn new(protocol: ProtocolVersion, response: Response) -> Self {
        let (local, mut remote) = vessel_rpc::memory::pair(NonZeroUsize::new(CAPACITY).unwrap());
        let node = NodeInfo {
            id: NodeId::random(),
            name: "removal-fixture".into(),
            hostname: "disposable".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            version: "0".into(),
            protocol: ProtocolRange::CURRENT,
            started_at: chrono::DateTime::UNIX_EPOCH,
            state: NodeState::Serving,
            public_key: None,
            timezone: None,
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recording = Arc::clone(&calls);
        let (send, seen) = mpsc::channel(CAPACITY);
        let task = tokio::spawn(async move {
            assert!(matches!(
                read(&mut remote).await,
                Some(ClientFrame::Hello(_))
            ));
            write(
                &mut remote,
                ServerFrame::Welcome(Welcome {
                    protocol,
                    node: node.clone(),
                }),
            )
            .await;
            while let Some(frame) = read(&mut remote).await {
                let ClientFrame::Request { id, call } = frame else {
                    continue;
                };
                recording.lock().unwrap().push(call.clone());
                send.send(call.clone()).await.unwrap();
                let outcome = if matches!(call, Call::RemoveAgent(_)) {
                    match response {
                        Response::Ack => Outcome::Ok(Reply::RemoveAgent(Ack {})),
                        Response::Error(code) => {
                            Outcome::Err(WireError::new(code, "fixture diagnostic"))
                        }
                        Response::Disconnect => break,
                        Response::WrongMethod => Outcome::Ok(Reply::NodeInfo(node.clone())),
                        Response::Wait => continue,
                    }
                } else if matches!(call, Call::NodeInfo(_)) {
                    Outcome::Ok(Reply::NodeInfo(node.clone()))
                } else {
                    panic!("unexpected fixture call")
                };
                write(&mut remote, ServerFrame::Response { id, outcome }).await;
            }
        });
        let client = NodeClient::over(
            local,
            ClientInfo {
                name: "removal-tests".into(),
                version: "0".into(),
            },
            &ClientConfig::default(),
        )
        .await
        .unwrap();
        Self {
            client,
            seen,
            calls,
            task,
        }
    }

    /// A read ordered after the tested action lets the peer drain prior writes.
    pub(super) async fn barrier(&self) {
        let _info = self.client.call(GetNode {}).await.unwrap();
    }

    pub(super) fn removals(&self) -> Vec<vessel_proto::agents::AgentName> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter_map(|call| {
                if let Call::RemoveAgent(request) = call {
                    Some(request.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
