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

#[derive(Clone)]
pub(crate) enum Behaviour {
    Success,
    Page(vessel_proto::history::AgentHistoryPage),
    Session(Box<SessionInfo>),
    Refuse(ErrorCode),
    Disconnect,
}

pub(crate) struct Peer {
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
            spec.spec,
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

pub(crate) async fn connect(behaviours: Vec<Behaviour>) -> Peer {
    connect_at(behaviours, vessel_proto::PROTOCOL_VERSION).await
}

pub(crate) async fn connect_at(
    behaviours: Vec<Behaviour>,
    protocol: vessel_proto::ProtocolVersion,
) -> Peer {
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
                ServerFrame::Welcome(Welcome { protocol, node })
            )
            .await
        );
        while let Some(frame) = read(&mut remote).await {
            if let ClientFrame::Request { id, call } = frame {
                recorded.lock().unwrap().push(call.clone());
                let outcome = match behaviours.pop_front().unwrap_or(Behaviour::Success) {
                    Behaviour::Success => result(call),
                    Behaviour::Page(page) => Outcome::Ok(Reply::GetAgentHistory(page)),
                    Behaviour::Session(info) => {
                        let reply = if matches!(call, Call::GetSession(_)) {
                            Reply::GetSession(*info)
                        } else {
                            assert!(
                                matches!(call, Call::ResumeSession(_)),
                                "unexpected activation call: {call:?}"
                            );
                            Reply::ResumeSession(*info)
                        };
                        Outcome::Ok(reply)
                    }
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

pub(crate) fn admitted(
    peer: &Peer,
) -> (
    crate::testing::Home,
    crate::State,
    crate::connections::SelectedConnection,
    vessel_app_view::connection::ConnectionContext,
    tokio::sync::mpsc::Receiver<crate::Event>,
) {
    let home = crate::testing::Home::new();
    let (mut state, events) = crate::testing::state(&home);
    let context = state
        .admissions
        .admit(
            vessel_app_view::devices::MachineKey::from_book("fixture"),
            peer.client.node().id,
        )
        .unwrap();
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let _previous = state.clients.insert(
        "fixture".into(),
        crate::Connected::new(peer.client.clone(), vec![], binding, context.clone()),
    );
    let selected = state.selected_connection(&context).unwrap();
    (home, state, selected, context, events)
}
