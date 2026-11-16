//! File-panel calls (protocol 27) reach a node that speaks them and never
//! enter a connection below it, the SDK wrappers included.
use super::*;
use crate::{CallError, WorkspaceFileCallError};
use vessel_proto::workspace::directory_metadata::{IgnoredEntries, InspectDirectory};
use vessel_proto::workspace::panel_files::{
    DuplicateFileEntry, FileOffset, ObserveFile, ReadFileBytes,
};
use vessel_proto::workspace::suggestions::{
    SuggestEntries, SuggestionKinds, SuggestionMatch, SuggestionQuery,
};
use vessel_proto::workspace::{RelPath, WorkspaceRoot};
use vessel_proto::{ClientFrame, ErrorCode, Method, Outcome, ProjectId, WireError};

/// The last protocol without the file-panel methods.
const OLD: ProtocolVersion = ProtocolVersion::new(26);
/// The protocol that assigned them (ADR 0105).
const NEEDS: ProtocolVersion = ProtocolVersion::new(27);
/// A hung framed peer must fail this fixture rather than hold a test worker.
const FIXTURE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
/// The wrappers in the order [`each_wrapper`] calls them.
const METHODS: [Method; 5] = [
    Method::ReadFileBytes,
    Method::InspectDirectory,
    Method::DuplicateFileEntry,
    Method::ObserveFile,
    Method::SuggestEntries,
];

/// A node speaking `protocol`; its task answers every request with a refusal
/// and returns the methods it received once the client hangs up.
async fn node_at(protocol: ProtocolVersion) -> (NodeClient, tokio::task::JoinHandle<Vec<Method>>) {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let serving = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol,
                    node: node(),
                }))
                .unwrap(),
            )
            .await
            .unwrap();
        let mut received = Vec::new();
        while let Some(frame) = node_end.next().await {
            let frame: ClientFrame = serde_json::from_str(&frame.unwrap()).unwrap();
            let ClientFrame::Request { id, call } = frame else {
                panic!("expected only requests after the handshake");
            };
            received.push(call.method());
            node_end
                .send(
                    serde_json::to_string(&ServerFrame::Response {
                        id,
                        outcome: Outcome::Err(WireError::new(ErrorCode::NotFound, "fixture")),
                    })
                    .unwrap(),
                )
                .await
                .unwrap();
        }
        received
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    (client, serving)
}

/// Every SDK wrapper once, its outcome beside the method it should send.
async fn each_wrapper(client: &NodeClient) -> Vec<(Result<(), WorkspaceFileCallError>, Method)> {
    let root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    let path: RelPath = "file".parse().unwrap();
    let outcomes = vec![
        client
            .read_workspace_byte_page(ReadFileBytes {
                root,
                path: path.clone(),
                offset: FileOffset::ZERO,
                expected: None,
            })
            .await
            .map(drop),
        client
            .inspect_workspace_directory(InspectDirectory {
                root,
                directory: RelPath::root(),
                ignored: IgnoredEntries::Include,
            })
            .await
            .map(drop),
        client
            .duplicate_workspace_entry(DuplicateFileEntry {
                root,
                path: path.clone(),
            })
            .await
            .map(drop),
        client
            .observe_workspace_file(ObserveFile {
                root,
                path,
                after: None,
            })
            .await
            .map(drop),
        client
            .suggest_workspace_entries(SuggestEntries {
                root,
                directory: RelPath::root(),
                query: SuggestionQuery::try_from(String::new()).unwrap(),
                kinds: SuggestionKinds::Both,
                matching: SuggestionMatch::Fuzzy,
                limit: None,
            })
            .await
            .map(drop),
    ];
    outcomes.into_iter().zip(METHODS).collect()
}

#[tokio::test]
async fn an_older_node_gets_no_file_panel_frame() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let (client, serving) = node_at(OLD).await;
        for (outcome, expected) in each_wrapper(&client).await {
            assert!(
                matches!(
                    outcome,
                    Err(WorkspaceFileCallError::Call(CallError::NotInProtocol {
                        method,
                        needs,
                        negotiated,
                    })) if method == expected && needs == NEEDS && negotiated == OLD
                ),
                "{expected:?}"
            );
        }
        drop(client);
        assert_eq!(serving.await.unwrap(), Vec::new(), "no file frame was sent");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_current_node_receives_each_file_panel_request() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let (client, serving) = node_at(vessel_proto::PROTOCOL_VERSION).await;
        for (outcome, expected) in each_wrapper(&client).await {
            assert!(
                matches!(
                    &outcome,
                    Err(WorkspaceFileCallError::Call(CallError::Remote(error)))
                        if error.code == ErrorCode::NotFound
                ),
                "{expected:?}: {outcome:?}"
            );
        }
        drop(client);
        assert_eq!(serving.await.unwrap(), METHODS);
    })
    .await
    .unwrap();
}
