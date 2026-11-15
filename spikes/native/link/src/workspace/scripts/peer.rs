//! In-memory peer speaks the real framed protocol; no node, disk or agent.
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange, Reply,
    ServerFrame, Welcome,
};

const FRAME_BYTES: usize = 65_536;
pub(in crate::workspace) struct Peer {
    pub(in crate::workspace) client: NodeClient,
    pub(in crate::workspace) calls: Arc<Mutex<Vec<Call>>>,
    task: tokio::task::JoinHandle<()>,
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
async fn write(stream: &mut DuplexStream, frame: ServerFrame) {
    let bytes = serde_json::to_vec(&frame).unwrap();
    stream
        .write_u32(u32::try_from(bytes.len()).unwrap())
        .await
        .unwrap();
    stream.write_all(&bytes).await.unwrap();
}
pub(in crate::workspace) async fn connect(
    answer: impl Fn(&Call) -> Result<Reply, vessel_proto::WireError> + Send + 'static,
) -> Peer {
    let (local, mut remote) = tokio::io::duplex(FRAME_BYTES);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::clone(&calls);
    let task = tokio::spawn(async move {
        assert!(matches!(
            read(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        write(
            &mut remote,
            ServerFrame::Welcome(Welcome {
                protocol: vessel_proto::PROTOCOL_VERSION,
                node: NodeInfo {
                    id: NodeId::random(),
                    name: "fake".to_owned(),
                    hostname: "fake".to_owned(),
                    os: "linux".to_owned(),
                    arch: "x86_64".to_owned(),
                    version: "0".to_owned(),
                    protocol: ProtocolRange::CURRENT,
                    started_at: chrono::DateTime::UNIX_EPOCH,
                    state: NodeState::Serving,
                    public_key: None,
                    timezone: None,
                },
            }),
        )
        .await;
        while let Some(frame) = read(&mut remote).await {
            if let ClientFrame::Request { id, call } = frame {
                recording.lock().unwrap().push(call.clone());
                let outcome = match answer(&call) {
                    Ok(reply) => Outcome::Ok(reply),
                    Err(error) => Outcome::Err(error),
                };
                write(&mut remote, ServerFrame::Response { id, outcome }).await;
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
        ClientInfo {
            name: "script-tests".to_owned(),
            version: "0".to_owned(),
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

impl Peer {
    /// Reuse the canonical actor admission factory, never a feature epoch cache.
    pub(in crate::workspace) fn selected(
        &self,
        name: &str,
    ) -> (
        crate::connections::SelectedConnection,
        crate::connections::Connected,
        vessel_app_view::connection::ConnectionContext,
    ) {
        let home = crate::testing::Home::new();
        let (mut actor, _events) = crate::testing::state(&home);
        let context = actor
            .admissions
            .admit(
                vessel_app_view::connection::MachineKey::from_book(name),
                self.client.node().id,
            )
            .unwrap();
        let connected = crate::connections::Connected::new(
            self.client.clone(),
            vec![],
            vessel_client::book::BookEntry {
                address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
                token: None,
                credential: None,
                id: None,
            },
            context.clone(),
        );
        let _old = actor.clients.insert(name.to_owned(), connected.clone());
        (
            actor.selected_connection(&context).unwrap(),
            connected,
            context,
        )
    }
}
