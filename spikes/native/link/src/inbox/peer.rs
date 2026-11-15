//! Bounded framed peer for inbox boundary tests; no app implementation here.
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Call, ClientFrame, ClientInfo, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    ServerFrame, Welcome,
};

pub(super) struct Peer {
    pub client: NodeClient,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded test-only frame reader; corrupt fixtures must fail the test"
)]
async fn read(stream: &mut DuplexStream) -> Option<ClientFrame> {
    let length = match stream.read_u32().await {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return None,
        Err(error) => panic!("test peer read: {error}"),
    };
    assert!(length <= 65_536);
    let mut bytes = vec![0; usize::try_from(length).unwrap()];
    let _read = stream.read_exact(&mut bytes).await.unwrap();
    Some(serde_json::from_slice(&bytes).unwrap())
}
#[expect(
    clippy::unwrap_used,
    reason = "test frame sizes and serialization are controlled by the fixture"
)]
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
#[expect(
    clippy::unwrap_used,
    reason = "test-only handshake and fixed frame capacity must be valid"
)]
pub(super) async fn connect(mut answer: impl FnMut(Call) -> Outcome + Send + 'static) -> Peer {
    let (local, mut remote) = tokio::io::duplex(65_536);
    let protocol = vessel_proto::Method::ListInbox.since().unwrap();
    let task = tokio::spawn(async move {
        assert!(matches!(
            read(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        let node = NodeInfo {
            id: NodeId::random(),
            name: "inbox-test".to_owned(),
            hostname: "test".to_owned(),
            os: "linux".to_owned(),
            arch: "x86_64".to_owned(),
            version: "0".to_owned(),
            protocol: ProtocolRange {
                min: protocol,
                max: protocol,
            },
            started_at: chrono::DateTime::UNIX_EPOCH,
            state: NodeState::Serving,
            public_key: None,
            timezone: None,
        };
        if !write(
            &mut remote,
            ServerFrame::Welcome(Welcome { protocol, node }),
        )
        .await
        {
            return;
        }
        while let Some(frame) = read(&mut remote).await {
            if let ClientFrame::Request { id, call } = frame {
                let outcome = answer(call);
                if !write(&mut remote, ServerFrame::Response { id, outcome }).await {
                    break;
                }
            }
        }
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(65_536).unwrap()),
        ClientInfo {
            name: "inbox-tests".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    Peer { client, task }
}
