//! Real framed RPC proves the diagnostic projection cannot execute discovery.
use super::*;
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_client::ClientConfig;
use vessel_proto::{
    Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome, ProtocolRange,
    Reply, ServerFrame, Welcome, WireError,
};
const FRAME_BYTES: usize = 65_536;
const CANARY: &str = "synthetic-private-value-never-export";
fn node(id: NodeId) -> NodeInfo {
    NodeInfo {
        id,
        name: CANARY.to_owned(),
        hostname: CANARY.to_owned(),
        os: CANARY.to_owned(),
        arch: CANARY.to_owned(),
        version: CANARY.to_owned(),
        protocol: ProtocolRange::CURRENT,
        started_at: chrono::DateTime::UNIX_EPOCH,
        state: NodeState::Serving,
        public_key: None,
        timezone: None,
    }
}
async fn read(stream: &mut DuplexStream) -> ClientFrame {
    let size = usize::try_from(stream.read_u32().await.unwrap()).unwrap();
    assert!(size <= FRAME_BYTES);
    let mut bytes = vec![0; size];
    assert_eq!(stream.read_exact(&mut bytes).await.unwrap(), bytes.len());
    serde_json::from_slice(&bytes).unwrap()
}
async fn write(stream: &mut DuplexStream, frame: ServerFrame) {
    let bytes = serde_json::to_vec(&frame).unwrap();
    stream
        .write_u32(u32::try_from(bytes.len()).unwrap())
        .await
        .unwrap();
    stream.write_all(&bytes).await.unwrap();
}
async fn peer(admitted: NodeId, outcome: Outcome) -> (NodeClient, tokio::task::JoinHandle<()>) {
    let (local, mut remote) = tokio::io::duplex(FRAME_BYTES);
    let task = tokio::spawn(async move {
        assert!(matches!(read(&mut remote).await, ClientFrame::Hello(_)));
        write(
            &mut remote,
            ServerFrame::Welcome(Welcome {
                protocol: vessel_proto::PROTOCOL_VERSION,
                node: node(admitted),
            }),
        )
        .await;
        if let ClientFrame::Request { id, call } = read(&mut remote).await {
            assert!(
                matches!(call, Call::NodeInfo(_)),
                "diagnostics must not discover a provider"
            );
            write(&mut remote, ServerFrame::Response { id, outcome }).await;
        } else {
            panic!("expected explicit node-info read");
        }
        let mut byte = [0];
        let _closed = remote.read(&mut byte).await;
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_BYTES).unwrap()),
        ClientInfo {
            name: "diagnostic-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    (client, task)
}
#[tokio::test]
async fn node_info_projects_only_allowed_fields_without_discovery() {
    let id = NodeId::random();
    let (client, task) = peer(id, Outcome::Ok(Reply::NodeInfo(node(id)))).await;
    let answer = collect(&client).await;
    assert_eq!(answer.admitted, id);
    assert_eq!(answer.facts.as_ref().unwrap().node, id);
    assert!(!format!("{answer:?}").contains(CANARY));
    task.abort();
}
#[tokio::test]
async fn peer_error_message_never_crosses_the_diagnostic_edge() {
    for (code, expected) in [
        (ErrorCode::Forbidden, Problem::Forbidden),
        (ErrorCode::Unsupported, Problem::Unsupported),
        (ErrorCode::Internal, Problem::Failed),
    ] {
        let (client, task) =
            peer(NodeId::random(), Outcome::Err(WireError::new(code, CANARY))).await;
        let answer = collect(&client).await;
        assert_eq!(answer.facts, Err(expected));
        assert!(!format!("{answer:?}").contains(CANARY));
        task.abort();
    }
}
#[test]
fn collector_capacity_and_queue_refusal_are_bounded() {
    let hub = Hub::default();
    let permits: Vec<_> = (0..MAX_PARALLEL_HOSTS)
        .map(|_| Arc::clone(&hub.0).try_acquire_owned().unwrap())
        .collect();
    assert!(Arc::clone(&hub.0).try_acquire_owned().is_err());
    drop(permits);
    assert!(Arc::clone(&hub.0).try_acquire_owned().is_ok());
    assert_eq!(super::super::rejected(crate::Rejected::Full), Problem::Busy);
    assert_eq!(
        super::super::rejected(crate::Rejected::Closed),
        Problem::Offline
    );
}

#[tokio::test]
async fn a_reply_for_another_node_is_not_attributed_to_the_admitted_host() {
    let admitted = NodeId::random();
    let (client, task) = peer(
        admitted,
        Outcome::Ok(Reply::NodeInfo(node(NodeId::random()))),
    )
    .await;
    let answer = collect(&client).await;
    assert_eq!(answer.admitted, admitted);
    assert_eq!(answer.facts, Err(Problem::InvalidReply));
    task.abort();
}
