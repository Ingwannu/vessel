//! Label requests must refuse unavailable capabilities before sending a frame.

use super::{client_info, node};
use crate::{CallError, NodeClient};
use futures_util::{SinkExt as _, StreamExt as _};
use std::num::NonZeroUsize;
use vessel_proto::{
    ErrorCode, MIN_PROTOCOL_VERSION, PROTOCOL_VERSION, ServerFrame, Welcome, WorkspaceId,
    workspace_labels::{
        AssignWorkspaceLabel, Assignment, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete, Label,
        LabelColor, ListWorkspaceLabels, UpdateWorkspaceLabel, WORKSPACE_LABELS,
    },
};

async fn refused<R: crate::Unary>(client: &NodeClient, request: R) {
    let error = match client.call(request).await {
        Ok(_) => panic!("an unavailable label request succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    assert!(matches!(error,
        CallError::UnassignedProtocol { method } | CallError::NotInProtocol { method, .. }
        if method == R::METHOD
    ));
}

#[tokio::test]
async fn workspace_labels_unassigned_or_older_peer_receives_no_request_frame() {
    for protocol in [MIN_PROTOCOL_VERSION, PROTOCOL_VERSION] {
        if WORKSPACE_LABELS.is_some_and(|since| protocol >= since) {
            continue;
        }
        let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
        let peer = tokio::spawn(async move {
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
            node_end.next().await
        });
        let client = NodeClient::over(client_end, client_info(), &crate::ClientConfig::default())
            .await
            .unwrap();
        refused(&client, ListWorkspaceLabels::default()).await;
        refused(
            &client,
            AssignWorkspaceLabel {
                workspace: WorkspaceId::random(),
                label: Label {
                    name: "Ready".parse().unwrap(),
                    color: LabelColor::Violet,
                },
                assignment: Assignment::Assigned,
            },
        )
        .await;
        refused(
            &client,
            UpdateWorkspaceLabel {
                name: "Ready".parse().unwrap(),
                new_name: Some("Done".parse().unwrap()),
                color: None,
            },
        )
        .await;
        refused(
            &client,
            InspectWorkspaceLabelDelete {
                name: "Ready".parse().unwrap(),
            },
        )
        .await;
        refused(
            &client,
            DeleteWorkspaceLabel {
                name: "Ready".parse().unwrap(),
            },
        )
        .await;
        drop(client);
        assert!(
            peer.await.unwrap().is_none(),
            "an unavailable request was sent"
        );
    }
}
