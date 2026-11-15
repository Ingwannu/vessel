//! A real `NodeClient` over the existing framed RPC transport. The peer only
//! answers protocol messages; it contains no book or app implementation.
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome,
    ProtocolRange, Reply, ServerFrame, SessionId, SessionInfo, SessionMode, SessionSpec,
    Subscribed, SubscriptionId, Welcome, WireError,
};

const FRAME_BYTES: usize = 65_536;
const CHANNEL_CAPACITY: usize = 4;

pub(crate) struct Peer {
    pub client: Option<NodeClient>,
    pub calls: Arc<Mutex<Vec<Call>>>,
    pub subscribed: mpsc::Receiver<()>,
    pub task: Option<JoinHandle<()>>,
    pub requests: Option<mpsc::Receiver<Request>>,
    frames: mpsc::Sender<Frame>,
}

async fn read(stream: &mut (impl AsyncRead + Unpin)) -> Option<ClientFrame> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return None,
        Err(error) => panic!("test peer read failed: {error}"),
    };
    assert!(
        usize::try_from(length).unwrap() <= FRAME_BYTES,
        "test frame exceeded its bound"
    );
    let mut bytes = vec![0; usize::try_from(length).unwrap()];
    assert_eq!(stream.read_exact(&mut bytes).await.unwrap(), bytes.len());
    Some(serde_json::from_slice(&bytes).unwrap())
}
async fn write(stream: &mut (impl AsyncWrite + Unpin), frame: ServerFrame) -> bool {
    let bytes = serde_json::to_vec(&frame).unwrap();
    if stream
        .write_u32(u32::try_from(bytes.len()).unwrap())
        .await
        .is_err()
    {
        return false;
    }
    stream.write_all(&bytes).await.is_ok()
}
pub(crate) fn info(session: SessionId) -> SessionInfo {
    SessionInfo::new(
        session,
        SessionSpec::new(
            vessel_proto::HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}
pub(crate) struct Request {
    pub call: Call,
    pub answer: tokio::sync::oneshot::Sender<Outcome>,
}

pub(crate) async fn connect(name: &str) -> Peer {
    start(name, None).await
}

pub(crate) async fn controlled(name: &str) -> Peer {
    let (send, receive) = mpsc::channel(CHANNEL_CAPACITY);
    let mut peer = start(name, Some(send)).await;
    peer.requests = Some(receive);
    peer
}

fn node(name: &str) -> NodeInfo {
    NodeInfo {
        id: NodeId::random(),
        name: name.to_owned(),
        hostname: "test-machine".to_owned(),
        os: "linux".to_owned(),
        arch: "x86_64".to_owned(),
        version: "0.1.0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}

async fn start(name: &str, control: Option<mpsc::Sender<Request>>) -> Peer {
    let (local, remote) = tokio::io::duplex(FRAME_BYTES);
    let (mut remote, mut writer) = tokio::io::split(remote);
    let (frames, mut outgoing) = mpsc::channel::<Frame>(CHANNEL_CAPACITY);
    let replies = frames.clone();
    let node = node(name);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::clone(&calls);
    let (ready, subscribed) = mpsc::channel(CHANNEL_CAPACITY);
    let task = tokio::spawn(async move {
        let reading = async move {
            assert!(matches!(
                read(&mut remote).await,
                Some(ClientFrame::Hello(_))
            ));
            assert!(
                send(
                    &replies,
                    ServerFrame::Welcome(Welcome {
                        protocol: vessel_proto::PROTOCOL_VERSION,
                        node
                    })
                )
                .await
            );
            while let Some(frame) = read(&mut remote).await {
                if let ClientFrame::Request { id, call } = frame {
                    recording.lock().unwrap().push(call.clone());
                    let reply = if let Some(control) = &control {
                        let (answer, receive) = tokio::sync::oneshot::channel();
                        if control.send(Request { call, answer }).await.is_err() {
                            break;
                        }
                        let Ok(outcome) = receive.await else {
                            break;
                        };
                        outcome
                    } else if matches!(&call, Call::ListSessions(_)) {
                        Outcome::Ok(Reply::ListSessions(Vec::new()))
                    } else if let Call::Subscribe(ask) = call {
                        let reply = Outcome::Ok(Reply::Subscribe(Subscribed {
                            subscription: SubscriptionId::new(1),
                            info: info(ask.session),
                        }));
                        ready.send(()).await.unwrap();
                        reply
                    } else if matches!(&call, Call::Unsubscribe(_)) {
                        Outcome::Ok(Reply::Unsubscribe(Ack {}))
                    } else {
                        Outcome::Err(WireError::new(
                            ErrorCode::Unsupported,
                            "not used by machine-book tests",
                        ))
                    };
                    if !send(&replies, ServerFrame::Response { id, outcome: reply }).await {
                        break;
                    }
                }
            }
        };
        let writing = async move {
            while let Some(Frame { frame, sent }) = outgoing.recv().await {
                let success = write(&mut writer, frame).await;
                let _receipt = sent.send(success);
                if !success {
                    break;
                }
            }
        };
        // One tracked owner: dropping the peer cancels both halves, no child task.
        tokio::select! { () = reading => {}, () = writing => {} }
    });
    // Own cleanup before the first handshake await: a failed phase drops the
    // Peer and aborts its one task, including both framed transport halves.
    let mut peer = Peer {
        client: None,
        calls,
        subscribed,
        task: Some(task),
        requests: None,
        frames,
    };
    let (reader, writer) = tokio::io::split(local);
    let client = super::phase(
        "framed peer handshake",
        NodeClient::over(
            vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
            ClientInfo {
                name: "machine-book-tests".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        ),
    )
    .await
    .unwrap();
    peer.client = Some(client);
    peer
}

struct Frame {
    frame: ServerFrame,
    sent: tokio::sync::oneshot::Sender<bool>,
}
async fn send(frames: &mpsc::Sender<Frame>, frame: ServerFrame) -> bool {
    let (sent, receipt) = tokio::sync::oneshot::channel();
    if frames.send(Frame { frame, sent }).await.is_err() {
        return false;
    }
    receipt.await.unwrap_or(false)
}
impl Peer {
    pub(crate) async fn push(&self, frame: ServerFrame) {
        assert!(super::phase("injected frame write receipt", send(&self.frames, frame)).await);
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
