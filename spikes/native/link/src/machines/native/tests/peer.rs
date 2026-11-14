//! A real `NodeClient` over the existing framed RPC transport. The peer only
//! answers protocol messages; it contains no book or app implementation.
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome,
    ProtocolRange, Reply, ServerFrame, SessionId, SessionInfo, SessionMode, SessionSpec,
    Subscribed, SubscriptionId, Welcome, WireError,
};

pub(super) struct Peer {
    pub client: Option<NodeClient>,
    pub calls: Arc<Mutex<Vec<Call>>>,
    pub subscribed: mpsc::Receiver<()>,
    pub task: Option<JoinHandle<()>>,
}

async fn read(stream: &mut DuplexStream) -> Option<ClientFrame> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return None,
        Err(error) => panic!("test peer read failed: {error}"),
    };
    assert!(length <= 65_536, "test frame exceeded its bound");
    let mut bytes = vec![0; usize::try_from(length).unwrap()];
    assert_eq!(stream.read_exact(&mut bytes).await.unwrap(), bytes.len());
    Some(serde_json::from_slice(&bytes).unwrap())
}
async fn write(stream: &mut DuplexStream, frame: ServerFrame) -> bool {
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
fn info(session: SessionId) -> SessionInfo {
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
pub(super) async fn connect(name: &str) -> Peer {
    let (local, mut remote) = tokio::io::duplex(65_536);
    let node = NodeInfo {
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
    };
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::clone(&calls);
    let (ready, subscribed) = mpsc::channel(4);
    let task = tokio::spawn(async move {
        assert!(matches!(
            read(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        assert!(
            write(
                &mut remote,
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
                let reply = if matches!(&call, Call::ListSessions(_)) {
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
                if !write(&mut remote, ServerFrame::Response { id, outcome: reply }).await {
                    break;
                }
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(65_536).unwrap()),
        ClientInfo {
            name: "machine-book-tests".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    Peer {
        client: Some(client),
        calls,
        subscribed,
        task: Some(task),
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
