//! The actual `NodeClient` framing against an in-memory peer; never a real node.

use super::*;
use chrono::DateTime;
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream};
use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::projects::actions::{ProjectActions, ProjectReply};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, ErrorCode, NodeId, NodeInfo, NodeState, Outcome,
    ProtocolRange, Reply as WireReply, ServerFrame, Welcome, WireError,
};

const FRAME_LIMIT: usize = 65536;
const TEST_WAIT: Duration = Duration::from_secs(10);

async fn read(stream: &mut DuplexStream) -> ClientFrame {
    let length = usize::try_from(stream.read_u32().await.unwrap()).unwrap();
    assert!(length <= FRAME_LIMIT);
    let mut bytes = vec![0; length];
    let _read = stream.read_exact(&mut bytes).await.unwrap();
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
async fn peer(outcome: Outcome) -> (NodeClient, tokio::task::JoinHandle<Call>) {
    let (local, mut remote) = tokio::io::duplex(FRAME_LIMIT);
    let peer = tokio::spawn(async move {
        assert!(matches!(read(&mut remote).await, ClientFrame::Hello(_)));
        write(
            &mut remote,
            ServerFrame::Welcome(Welcome {
                protocol: vessel_proto::PROTOCOL_VERSION,
                node: NodeInfo {
                    id: NodeId::random(),
                    name: "fixture".to_owned(),
                    hostname: "fixture".to_owned(),
                    os: "linux".to_owned(),
                    arch: "x86_64".to_owned(),
                    version: "0".to_owned(),
                    protocol: ProtocolRange::CURRENT,
                    started_at: DateTime::UNIX_EPOCH,
                    state: NodeState::Serving,
                    public_key: None,
                    timezone: None,
                },
            }),
        )
        .await;
        let ClientFrame::Request { id, call } = read(&mut remote).await else {
            panic!("expected unary mutation")
        };
        write(&mut remote, ServerFrame::Response { id, outcome }).await;
        call
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(FRAME_LIMIT).unwrap()),
        ClientInfo {
            name: "project-actions-test".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    (client, peer)
}

#[tokio::test]
async fn the_real_remove_rpc_returns_its_exact_form_key_and_target() {
    tokio::time::timeout(TEST_WAIT, async {
        let (client, peer) = peer(Outcome::Ok(WireReply::RemoveProject(Ack))).await;
        let project = vessel_proto::ProjectId::random();
        let mut tracker = ProjectActions::default();
        let key = tracker.begin(Name::parse("laptop").unwrap()).unwrap();
        let (events, _received) = tokio::sync::mpsc::channel(1);
        let reply = Reply {
            machine: "laptop".to_owned(),
            events,
        };
        let answer = run(
            &Hub::default(),
            &client,
            &reply,
            ProjectChange {
                key: key.clone(),
                action: ProjectAction::Remove(project),
            },
        )
        .await;
        let Answer::ProjectChanged {
            key: returned,
            result,
        } = answer
        else {
            panic!("wrong answer kind")
        };
        assert_eq!(returned, key);
        assert_eq!(result, Ok(ProjectChanged::Removed(project)));
        assert_eq!(tracker.settle(&returned), ProjectReply::Current);
        assert_eq!(
            peer.await.unwrap(),
            Call::RemoveProject(RemoveProject { project })
        );
    })
    .await
    .expect("bounded in-memory RPC");
}

#[test]
fn unsent_dispatch_preserves_correlation_and_never_reports_removal_success() {
    let mut tracker = ProjectActions::default();
    let key = tracker.begin(Name::parse("laptop").unwrap()).unwrap();
    let answer = super::super::Ask::ProjectChange(ProjectChange {
        key: key.clone(),
        action: ProjectAction::Remove(vessel_proto::ProjectId::random()),
    })
    .refused(&Problem::Unsent);
    let Answer::ProjectChanged {
        key: returned,
        result,
    } = answer
    else {
        panic!("wrong answer kind")
    };
    assert_eq!(returned, key);
    assert_eq!(result, Err(ProjectFailure::Call(Problem::Unsent)));
    assert_eq!(tracker.settle(&returned), ProjectReply::Current);
}

#[tokio::test]
async fn a_readonly_node_refusal_is_not_an_acknowledged_change() {
    tokio::time::timeout(TEST_WAIT, async {
        let (client, peer) = peer(Outcome::Err(WireError::new(
            ErrorCode::Forbidden,
            "read-only device",
        )))
        .await;
        let mut tracker = ProjectActions::default();
        let key = tracker.begin(Name::parse("laptop").unwrap()).unwrap();
        let (events, _received) = tokio::sync::mpsc::channel(1);
        let reply = Reply {
            machine: "laptop".to_owned(),
            events,
        };
        let answer = run(
            &Hub::default(),
            &client,
            &reply,
            ProjectChange {
                key,
                action: ProjectAction::Remove(vessel_proto::ProjectId::random()),
            },
        )
        .await;
        let Answer::ProjectChanged { result, .. } = answer else {
            panic!("wrong answer kind")
        };
        assert!(matches!(
            result,
            Err(ProjectFailure::Call(Problem::Node {
                code: ErrorCode::Forbidden,
                ..
            }))
        ));
        assert!(matches!(peer.await.unwrap(), Call::RemoveProject(_)));
    })
    .await
    .expect("bounded in-memory RPC");
}
