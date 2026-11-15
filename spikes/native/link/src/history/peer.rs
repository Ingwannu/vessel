//! Test-only peer speaks real framed RPC entirely in memory.

use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    Reply, ServerFrame, SessionId, SessionInfo, Welcome, WireError,
};

const FRAME_LIMIT: usize = 65_536;

#[derive(Clone, Copy)]
pub(super) enum Behaviour {
    Success,
    Refuse(ErrorCode),
    Disconnect,
}

pub(super) struct Peer {
    pub client: NodeClient,
    pub calls: Arc<Mutex<Vec<Call>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read(stream: &mut DuplexStream) -> Option<ClientFrame> {
    let size = match stream.read_u32().await {
        Ok(size) => usize::try_from(size).unwrap(),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return None,
        Err(error) => panic!("in-memory peer read: {error}"),
    };
    assert!(size <= FRAME_LIMIT);
    let mut bytes = vec![0; size];
    assert_eq!(stream.read_exact(&mut bytes).await.unwrap(), size);
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
fn result(call: Call) -> Outcome {
    if let Call::CreateSession(spec) = call {
        Outcome::Ok(Reply::CreateSession(SessionInfo::new(
            SessionId::random(),
            spec,
            chrono::DateTime::UNIX_EPOCH,
        )))
    } else if let Call::ListProfiles(_) = call {
        Outcome::Ok(Reply::ListProfiles(Vec::new()))
    } else {
        Outcome::Err(WireError::new(
            ErrorCode::Unsupported,
            "test peer refuses other methods",
        ))
    }
}

pub(super) async fn connect(behaviours: Vec<Behaviour>) -> Peer {
    let (local, mut remote) = tokio::io::duplex(FRAME_LIMIT);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&calls);
    let mut behaviours: VecDeque<_> = behaviours.into();
    let task = tokio::spawn(async move {
        assert!(matches!(
            read(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        let node = NodeInfo {
            id: NodeId::random(),
            name: "fixture".into(),
            hostname: "fixture".into(),
            os: "test".into(),
            arch: "test".into(),
            version: "0".into(),
            protocol: ProtocolRange::CURRENT,
            started_at: chrono::DateTime::UNIX_EPOCH,
            state: NodeState::Serving,
            public_key: None,
            timezone: None,
        };
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
                recorded.lock().unwrap().push(call.clone());
                let outcome = match behaviours.pop_front().unwrap_or(Behaviour::Success) {
                    Behaviour::Success => result(call),
                    Behaviour::Refuse(code) => {
                        Outcome::Err(WireError::new(code, "untrusted peer message"))
                    }
                    Behaviour::Disconnect => break,
                };
                if !write(&mut remote, ServerFrame::Response { id, outcome }).await {
                    break;
                }
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_LIMIT).unwrap()),
        ClientInfo {
            name: "history-test".into(),
            version: "0".into(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    Peer {
        client,
        calls,
        task,
    }
}
