//! Label requests at protocol 26 (ADR 0086): an older peer receives no frame;
//! an assigned peer receives each call and the client reads its answer.

use super::{client_info, node};
use crate::{CallError, NodeClient, UnaryCall};
use futures_util::{SinkExt as _, StreamExt as _};
use std::num::NonZeroUsize;
use vessel_proto::{
    Call, ClientFrame, ErrorCode, MIN_PROTOCOL_VERSION, Method, Outcome, PROTOCOL_VERSION,
    ProtocolVersion, ServerFrame, Welcome, WireError, WorkspaceId,
    workspace_labels::{
        AssignWorkspaceLabel, Assignment, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete, Label,
        LabelColor, ListWorkspaceLabels, UpdateWorkspaceLabel, WORKSPACE_LABELS,
    },
};

// A hung framed peer must fail this fixture rather than retain a test worker.
const LABEL_SDK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

/// The five calls, in the order the fixtures send them.
fn calls() -> [Call; 5] {
    [
        vessel_proto::Request::into_call(ListWorkspaceLabels::default()),
        vessel_proto::Request::into_call(AssignWorkspaceLabel {
            workspace: WorkspaceId::random(),
            label: Label {
                name: "Ready".parse().unwrap(),
                color: LabelColor::Violet,
            },
            assignment: Assignment::Assigned,
        }),
        vessel_proto::Request::into_call(UpdateWorkspaceLabel {
            name: "Ready".parse().unwrap(),
            new_name: Some("Done".parse().unwrap()),
            color: None,
        }),
        vessel_proto::Request::into_call(InspectWorkspaceLabelDelete {
            name: "Ready".parse().unwrap(),
        }),
        vessel_proto::Request::into_call(DeleteWorkspaceLabel {
            name: "Ready".parse().unwrap(),
        }),
    ]
}

/// A connected client and the node end of its transport, at `protocol`.
async fn connected(protocol: ProtocolVersion) -> (NodeClient, vessel_rpc::memory::MemoryTransport) {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let handshake = async move {
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
        node_end
    };
    let config = crate::ClientConfig::default();
    let (client, node_end) = tokio::join!(
        NodeClient::over(client_end, client_info(), &config),
        handshake
    );
    (client.unwrap(), node_end)
}

#[tokio::test]
async fn workspace_labels_older_peer_receives_no_request_frame() {
    let since = WORKSPACE_LABELS.expect("protocol 26 assigns the label calls");
    let previous = ProtocolVersion::new(since.get().saturating_sub(1));
    for protocol in [MIN_PROTOCOL_VERSION, previous] {
        let (client, mut node_end) = connected(protocol).await;
        for call in calls() {
            let method = call.method();
            let Err(error) = client.call_any(UnaryCall::try_from(call).unwrap()).await else {
                panic!("a label request reached a protocol {protocol} peer");
            };
            assert_eq!(error.code(), Some(ErrorCode::Unsupported));
            assert!(matches!(error,
                CallError::NotInProtocol { method: refused, .. } if refused == method
            ));
        }
        drop(client);
        assert!(
            node_end.next().await.is_none(),
            "an unavailable request was sent"
        );
    }
}

#[tokio::test]
async fn workspace_labels_assigned_peer_receives_each_call_and_its_answer() {
    assert_eq!(WORKSPACE_LABELS, Some(PROTOCOL_VERSION));
    tokio::time::timeout(LABEL_SDK_DEADLINE, async {
        let (client, mut node_end) = connected(PROTOCOL_VERSION).await;
        for call in calls() {
            let method = call.method();
            let answer = async {
                let text = node_end.next().await.unwrap().unwrap();
                let ClientFrame::Request { id, call } = serde_json::from_str(&text).unwrap() else {
                    panic!("expected the label request frame");
                };
                assert_eq!(call.method(), method);
                node_end
                    .send(
                        serde_json::to_string(&ServerFrame::Response {
                            id,
                            outcome: Outcome::Err(WireError::new(
                                ErrorCode::NotFound,
                                "fixture answer",
                            )),
                        })
                        .unwrap(),
                    )
                    .await
                    .unwrap();
            };
            let unary = UnaryCall::try_from(call).unwrap();
            let (result, ()) = tokio::join!(client.call_any(unary), answer);
            let Err(CallError::Remote(error)) = result else {
                panic!("the node's answer to {method:?} was not read");
            };
            assert_eq!(error.code, ErrorCode::NotFound);
        }
        assert!(matches!(
            Method::ListWorkspaceLabels.since(),
            Some(since) if since == PROTOCOL_VERSION
        ));
    })
    .await
    .unwrap();
}
