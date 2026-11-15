//! Pending file APIs reject before a transport frame, including SDK wrappers.
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
use vessel_proto::{Method, ProjectId};

async fn pending_node() -> (NodeClient, tokio::task::JoinHandle<bool>) {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let serving = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol: vessel_proto::PROTOCOL_VERSION,
                    node: node(),
                }))
                .unwrap(),
            )
            .await
            .unwrap();
        node_end.next().await.is_none()
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    (client, serving)
}

#[tokio::test]
async fn pending_workspace_file_wrappers_send_no_frame() {
    let (client, serving) = pending_node().await;
    let root = WorkspaceRoot::Project {
        project: ProjectId::random(),
    };
    let path: RelPath = "file".parse().unwrap();
    let outcomes = [
        (
            client
                .read_workspace_byte_page(ReadFileBytes {
                    root,
                    path: path.clone(),
                    offset: FileOffset::ZERO,
                    expected: None,
                })
                .await
                .map(drop),
            Method::ReadFileBytes,
        ),
        (
            client
                .inspect_workspace_directory(InspectDirectory {
                    root,
                    directory: RelPath::root(),
                    ignored: IgnoredEntries::Include,
                })
                .await
                .map(drop),
            Method::InspectDirectory,
        ),
        (
            client
                .duplicate_workspace_entry(DuplicateFileEntry {
                    root,
                    path: path.clone(),
                })
                .await
                .map(drop),
            Method::DuplicateFileEntry,
        ),
        (
            client
                .observe_workspace_file(ObserveFile {
                    root,
                    path,
                    after: None,
                })
                .await
                .map(drop),
            Method::ObserveFile,
        ),
        (
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
            Method::SuggestEntries,
        ),
    ];
    for (outcome, expected) in outcomes {
        assert!(
            matches!(outcome, Err(WorkspaceFileCallError::Call(CallError::UnassignedProtocol { method })) if method == expected)
        );
    }
    drop(client);
    assert!(serving.await.unwrap(), "no file frame was sent");
}
