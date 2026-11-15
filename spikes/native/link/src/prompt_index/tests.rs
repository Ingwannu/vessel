//! A real typed client talks to an in-memory framed peer; it must send no
//! pending/new method before admission. No network, config, credentials or agent.
use super::*;
use std::num::NonZeroUsize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::mpsc;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::prompt_index::{IndexTarget, PromptIndex};
use vessel_client::{ClientConfig, NodeClient};
use vessel_proto::{
    ClientInfo, NodeId, NodeInfo, NodeState, ProtocolRange, ProtocolVersion, ServerFrame,
    SessionId, Welcome,
};

fn ask() -> IndexAsk {
    PromptIndex::default()
        .refresh(
            IndexTarget {
                machine: MachineKey::from_book("fixture"),
                connection: None,
                session: SessionId::random(),
            },
            None,
        )
        .unwrap()
}

#[tokio::test]
async fn disconnected_and_full_queues_settle_through_the_typed_failure_boundary() {
    let hub = Hub::default();
    let (events, mut answers) = mpsc::channel(1);
    hub.ask(
        Err(vessel_app_view::connection::Refusal::Changed),
        ask(),
        events,
    )
    .await;
    let crate::Event::PromptIndex(answer) = answers.recv().await.unwrap() else {
        panic!("wrong reply");
    };
    assert_eq!(answer.result, Err(IndexProblem::Connection));
    assert_eq!(rejected(crate::Rejected::Full), IndexProblem::Busy);
    assert_eq!(rejected(crate::Rejected::Closed), IndexProblem::Connection);
}

#[tokio::test]
async fn an_unadmitted_call_sends_no_frame_and_keeps_the_original_target_and_ticket() {
    let (local, mut remote) = tokio::io::duplex(65_536);
    let negotiated = vessel_proto::prompt_index::PROMPT_INDEX_PROTOCOL
        .map_or(vessel_proto::PROTOCOL_VERSION, |version| {
            ProtocolVersion::new(version.get().checked_sub(1).unwrap())
        });
    let peer = tokio::spawn(async move {
        let size = usize::try_from(remote.read_u32().await.unwrap()).unwrap();
        assert!(size < 65_536);
        let mut hello = vec![0; size];
        assert_eq!(remote.read_exact(&mut hello).await.unwrap(), hello.len());
        assert!(matches!(
            serde_json::from_slice::<vessel_proto::ClientFrame>(&hello).unwrap(),
            vessel_proto::ClientFrame::Hello(_)
        ));
        let frame = ServerFrame::Welcome(Welcome {
            protocol: negotiated,
            node: NodeInfo {
                id: NodeId::random(),
                name: "fixture".to_owned(),
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
        });
        let bytes = serde_json::to_vec(&frame).unwrap();
        remote
            .write_u32(u32::try_from(bytes.len()).unwrap())
            .await
            .unwrap();
        remote.write_all(&bytes).await.unwrap();
        remote.read_u32().await
    });
    let (reader, writer) = tokio::io::split(local);
    let client = NodeClient::over(
        vessel_rpc::Framed::new(reader, writer, NonZeroUsize::new(65_536).unwrap()),
        ClientInfo {
            name: "prompt-index-tests".to_owned(),
            version: "0".to_owned(),
        },
        &ClientConfig::default(),
    )
    .await
    .unwrap();
    let home = crate::testing::Home::new();
    std::fs::write(
        home.book(),
        "[nodes.fixture]\nurl = \"ws://127.0.0.1:0/rpc\"\n",
    )
    .unwrap();
    let (mut state, mut answers) = crate::testing::state(&home);
    let key = MachineKey::from_book("fixture");
    let node = client.node().id;
    let context = state.admissions.admit(key.clone(), node).unwrap();
    let binding = state
        .saved_entries()
        .await
        .unwrap()
        .into_iter()
        .find(|(name, _)| name.as_str() == "fixture")
        .unwrap()
        .1;
    assert!(
        state
            .clients
            .insert(
                "fixture".to_owned(),
                crate::Connected::new(client, Vec::new(), binding, context.clone())
            )
            .is_none()
    );
    let mut request = ask();
    request.target.connection = Some(context.clone());
    let other_node = state
        .admissions
        .admit(key.clone(), NodeId::random())
        .unwrap();
    let stale_epoch = state.admissions.admit(key, node).unwrap();
    for connection in [None, Some(other_node), Some(stale_epoch)] {
        let mut wrong = request.clone();
        wrong.target.connection = connection;
        state.prompt_index(wrong).await;
        let crate::Event::PromptIndex(answer) = answers.recv().await.unwrap() else {
            panic!("wrong reply");
        };
        assert_eq!(answer.result, Err(IndexProblem::Connection));
    }
    let mut wrong_alias = request.clone();
    wrong_alias.target.machine = MachineKey::from_book("another-alias");
    state.prompt_index(wrong_alias).await;
    let crate::Event::PromptIndex(answer) = answers.recv().await.unwrap() else {
        panic!("wrong reply");
    };
    assert_eq!(answer.result, Err(IndexProblem::Connection));
    state.prompt_index(request.clone()).await;
    let crate::Event::PromptIndex(answer) = answers.recv().await.unwrap() else {
        panic!("wrong reply");
    };
    assert_eq!(answer.ask.ticket, request.ticket);
    assert_eq!(answer.ask.target, request.target);
    assert_eq!(answer.result, Err(IndexProblem::Unsupported));
    drop(state);
    assert_eq!(
        peer.await.unwrap().unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
