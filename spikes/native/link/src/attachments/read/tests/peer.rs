//! A scripted framed peer uses only an in-process byte stream, never a socket.
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use tokio::task::JoinHandle;
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    BlobChunk, Call, ClientFrame, ClientInfo, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    Reply, ServerFrame, Welcome,
};

/// Test liveness is independent of the product's 30-second read deadline.
const TEST_SAFETY: std::time::Duration = std::time::Duration::from_secs(45);

pub(in crate::attachments) async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(TEST_SAFETY, future)
        .await
        .expect("test safety deadline")
}

/// A failed assertion/receive also tears down the peer owned by this test.
pub(in crate::attachments) struct Peer(JoinHandle<Vec<Call>>);
impl Drop for Peer {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl Peer {
    pub(in crate::attachments) async fn finish(mut self) -> Vec<Call> {
        if let Ok(result) = tokio::time::timeout(TEST_SAFETY, &mut self.0).await {
            result.expect("scripted peer completed")
        } else {
            self.0.abort();
            let _cancelled = (&mut self.0).await;
            panic!("scripted peer shutdown deadline");
        }
    }
}

const FRAME_BOUND: usize = 16 * 1024;

async fn receive(stream: &mut DuplexStream) -> Option<ClientFrame> {
    bounded(receive_frame(stream)).await
}

async fn receive_frame(stream: &mut DuplexStream) -> Option<ClientFrame> {
    let length = match stream.read_u32().await {
        Ok(size) => usize::try_from(size).unwrap(),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return None,
        Err(error) => panic!("peer input: {error}"),
    };
    assert!(length <= FRAME_BOUND);
    let mut bytes = vec![0; length];
    let _received = stream.read_exact(&mut bytes).await.unwrap();
    Some(serde_json::from_slice(&bytes).unwrap())
}

async fn send(stream: &mut DuplexStream, frame: ServerFrame) {
    bounded(send_frame(stream, frame)).await;
}

async fn send_frame(stream: &mut DuplexStream, frame: ServerFrame) {
    let bytes = serde_json::to_vec(&frame).unwrap();
    stream
        .write_u32(u32::try_from(bytes.len()).unwrap())
        .await
        .unwrap();
    stream.write_all(&bytes).await.unwrap();
}

pub(in crate::attachments) async fn connect(chunks: Vec<BlobChunk>) -> (NodeClient, Peer) {
    let (local, mut remote) = tokio::io::duplex(FRAME_BOUND);
    let task = tokio::spawn(async move {
        assert!(matches!(
            receive(&mut remote).await,
            Some(ClientFrame::Hello(_))
        ));
        send(
            &mut remote,
            ServerFrame::Welcome(Welcome {
                protocol: vessel_proto::PROTOCOL_VERSION,
                node: NodeInfo {
                    id: NodeId::random(),
                    name: "image-tests".to_owned(),
                    hostname: "fixture".to_owned(),
                    os: "test".to_owned(),
                    arch: "test".to_owned(),
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
        let mut calls = Vec::new();
        for chunk in chunks {
            let Some(ClientFrame::Request { id, call }) = receive(&mut remote).await else {
                panic!("expected an attachment request");
            };
            assert!(matches!(&call, Call::ReadAttachment(_)));
            calls.push(call);
            send(
                &mut remote,
                ServerFrame::Response {
                    id,
                    outcome: Outcome::Ok(Reply::ReadAttachment(chunk)),
                },
            )
            .await;
        }
        // A guarded result is observed only while its admitted transport lives.
        // Keep the peer open until the test drops its actor/selected handle.
        assert!(
            receive(&mut remote).await.is_none(),
            "unexpected request after scripted replies"
        );
        calls
    });
    let task = Peer(task);
    let (reader, writer) = tokio::io::split(local);
    let client = bounded(NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_BOUND).unwrap()),
        ClientInfo {
            name: "image-read-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    ))
    .await
    .unwrap();
    (client, task)
}

#[tokio::test]
async fn dropping_the_owned_peer_guard_cancels_a_stalled_script() {
    let (released, incoming) = tokio::sync::oneshot::channel::<()>();
    let task = Peer(tokio::spawn(async move {
        let _released = released;
        std::future::pending::<()>().await;
        Vec::new()
    }));
    tokio::task::yield_now().await;
    drop(task);
    assert!(bounded(incoming).await.is_err());
}
