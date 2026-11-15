//! In-memory framed peer exercises the real `NodeClient` without a socket/provider.
use std::num::NonZeroUsize;
const FRAME_CAPACITY: usize = 65_536;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream},
    sync::mpsc,
};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    Reply, ServerFrame, SessionId, SessionInfo, SessionMode, SessionSpec, Welcome, WireError,
};
#[derive(Clone, Copy)]
pub(super) enum Behavior {
    Accept,
    Refuse,
    WrongSession,
    Disconnect,
}
pub(super) struct Peer {
    pub client: NodeClient,
    pub calls: mpsc::Receiver<Call>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn read(stream: &mut DuplexStream) -> Option<ClientFrame> {
    let length = stream.read_u32().await.ok()?;
    let size = usize::try_from(length).unwrap();
    assert!(size <= FRAME_CAPACITY);
    let mut data = vec![0; size];
    let _read = stream.read_exact(&mut data).await.unwrap();
    Some(serde_json::from_slice(&data).unwrap())
}
async fn write(stream: &mut DuplexStream, frame: ServerFrame) -> bool {
    let data = serde_json::to_vec(&frame).unwrap();
    stream
        .write_u32(u32::try_from(data.len()).unwrap())
        .await
        .is_ok()
        && stream.write_all(&data).await.is_ok()
}
fn info(id: SessionId) -> SessionInfo {
    SessionInfo::new(
        id,
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
pub(super) async fn connect(behavior: Behavior) -> Peer {
    connect_as(behavior, NodeId::random()).await
}
pub(super) async fn connect_as(behavior: Behavior, node: NodeId) -> Peer {
    let (local, mut remote) = tokio::io::duplex(FRAME_CAPACITY);
    let node = NodeInfo {
        id: node,
        name: "peer".to_owned(),
        hostname: "test".to_owned(),
        os: "test".to_owned(),
        arch: "test".to_owned(),
        version: "0".to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    };
    let (sender, calls) = mpsc::channel(8);
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
                sender.send(call.clone()).await.unwrap();
                let outcome = match behavior {
                    Behavior::Disconnect => break,
                    Behavior::Refuse => {
                        Outcome::Err(WireError::new(ErrorCode::Conflict, "synthetic refusal"))
                    }
                    Behavior::WrongSession => {
                        Outcome::Ok(Reply::GetSession(info(SessionId::random())))
                    }
                    Behavior::Accept => {
                        if let Call::GetSession(request) = call {
                            Outcome::Ok(Reply::GetSession(info(request.session)))
                        } else if let Call::MarkSession(request) = call {
                            let mut current = info(request.session);
                            if request.mark == vessel_proto::Mark::Archive {
                                current.marks.archived_at = Some(chrono::DateTime::UNIX_EPOCH);
                                current.status = vessel_proto::SessionStatus::Interrupted;
                            }
                            Outcome::Ok(Reply::MarkSession(current))
                        } else {
                            Outcome::Err(WireError::new(
                                ErrorCode::Unsupported,
                                "unexpected fixture request",
                            ))
                        }
                    }
                };
                if !write(&mut remote, ServerFrame::Response { id, outcome }).await {
                    break;
                }
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = tokio::time::timeout(
        super::TEST_WAIT,
        NodeClient::over(
            vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_CAPACITY).unwrap()),
            ClientInfo {
                name: "session-tests".to_owned(),
                version: "0".to_owned(),
            },
            &ClientConfig::default(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    Peer {
        client,
        calls,
        task,
    }
}
