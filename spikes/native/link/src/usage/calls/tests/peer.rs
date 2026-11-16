//! Fixture-only framed peer: no node process, filesystem, network or credentials.
use std::num::NonZeroUsize;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream},
    task::JoinHandle,
};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    ServerFrame, Welcome,
};
const FRAME_BYTES: usize = 65_536;
pub(super) struct Peer {
    pub client: crate::connections::SelectedConnection,
    pub state: crate::State,
    _home: crate::testing::Home,
    task: JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn read(stream: &mut DuplexStream) -> Option<ClientFrame> {
    let length = stream.read_u32().await.ok()?;
    let length = usize::try_from(length).unwrap();
    assert!(length <= FRAME_BYTES);
    let mut bytes = vec![0; length];
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
pub(super) async fn connect(mut reply: impl FnMut(Call) -> Outcome + Send + 'static) -> Peer {
    let (local, mut remote) = tokio::io::duplex(FRAME_BYTES);
    let task = tokio::spawn(async move {
        assert!(matches!(
            read(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        let node = NodeInfo {
            id: NodeId::random(),
            name: "fixture".into(),
            hostname: "fixture".into(),
            os: "fixture".into(),
            arch: "fixture".into(),
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
            if let ClientFrame::Request { id, call } = frame
                && !write(
                    &mut remote,
                    ServerFrame::Response {
                        id,
                        outcome: reply(call),
                    },
                )
                .await
            {
                break;
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
        ClientInfo {
            name: "usage-fixture".into(),
            version: "0".into(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    let home = crate::testing::Home::new();
    let (mut state, _events) = crate::testing::state(&home);
    let context = state
        .admissions
        .admit(
            vessel_app_view::connection::MachineKey::from_book("fixture"),
            client.node().id,
        )
        .unwrap();
    let binding = vessel_client::book::BookEntry {
        address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
        token: None,
        credential: None,
        id: None,
    };
    let connected =
        crate::connections::Connected::new(client, Vec::new(), binding, context.clone());
    let _previous = state.clients.insert("fixture".into(), connected);
    let client = state.selected_connection(&context).unwrap();
    Peer {
        client,
        task,
        state,
        _home: home,
    }
}
