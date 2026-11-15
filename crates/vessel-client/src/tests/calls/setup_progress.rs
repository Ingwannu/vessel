//! Pending methods are refused even on the newest negotiated connection.

use super::*;
use vessel_proto::workspace::{CancelSetup, ReadSetupProgress};
use vessel_proto::{SetupRunId, WorktreeId};

#[tokio::test]
async fn unassigned_setup_progress_and_cancel_never_reach_the_wire() {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::new(4).unwrap());
    let serving = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        let welcome = ServerFrame::Welcome(Welcome {
            protocol: vessel_proto::PROTOCOL_VERSION,
            node: node(),
        });
        node_end
            .send(serde_json::to_string(&welcome).unwrap())
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let worktree = WorktreeId::random();
    let error = client
        .call(ReadSetupProgress {
            worktree,
            after: None,
            wait_ms: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    let error = client
        .call(CancelSetup {
            worktree,
            run: SetupRunId::random(),
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Unsupported));
    drop(client);
    assert!(
        serving.await.unwrap().is_none(),
        "no pending operation was sent"
    );
}
