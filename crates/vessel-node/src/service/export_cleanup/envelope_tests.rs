//! Force the actual Forward/Relay refusal arms through an unprojected raw peer.
//! Actual callback transport has its separate fixture; no assigned fleet is claimed.

use super::{fixture_peer::FixturePeer, runtime_tests::CallbackFleet};
use std::time::Duration;
use tokio_util::task::TaskTracker;
use vessel_client::{CallError, NodeClient};
use vessel_proto::{
    ErrorCode, ExportSession, GetNode, OriginBy, Request as _, Scope, SessionId, TransferManifest,
};

const ENVELOPE_TEST_BUDGET: Duration = Duration::from_secs(60);

enum Completion {
    Wait,
    Abort,
}

fn refused(result: Result<TransferManifest, CallError>) {
    let Err(CallError::Remote(error)) = result else {
        panic!("expected exact metadata refusal");
    };
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert_eq!(
        error.message,
        CallError::UnassignedProtocol {
            method: ExportSession::METHOD
        }
        .to_string()
    );
}

async fn finish_call(
    completion: Completion,
    caller: tokio::task::JoinHandle<Result<TransferManifest, CallError>>,
    peer: &mut FixturePeer,
    owner: &NodeClient,
    tasks: &TaskTracker,
) {
    assert_eq!(
        tasks.len(),
        1,
        "the actual refusal arm must retain a separate cleanup task"
    );
    match completion {
        Completion::Wait => {
            peer.release();
            refused(caller.await.unwrap());
        }
        Completion::Abort => {
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
            // Queue a normal request after the SDK's cancellation, without a sleep.
            let _barrier = owner.call(GetNode).await.unwrap();
            assert_eq!(
                tasks.len(),
                1,
                "cancelling the caller must not discard held cleanup"
            );
            peer.release();
            let _first_close = tasks.close();
            tasks.wait().await;
        }
    }
    assert!(tasks.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_forward_and_relay_metadata_refusals_settle_after_caller_abort() {
    vessel_testkit::within(ENVELOPE_TEST_BUDGET, async {
        let fleet = CallbackFleet::start().await;
        let home = &fleet.home().secure.service.0;
        // This fixture owns its cache entries; background membership reconciliation
        // must not replace the deliberately raw peer while a close is held.
        home.fleet.membership_epoch().cancel();
        for node in [fleet.home(), fleet.member()] {
            let owner = fleet.owner(node).await;
            for completion in [Completion::Wait, Completion::Abort] {
                let (client, mut peer) =
                    FixturePeer::start(fleet.home().info(), fleet.home().info().id).await;
                let target = client.node().id;
                home.links.connected_member_for_test(client).await;
                let sending = owner.clone();
                let caller = tokio::spawn(async move {
                    sending
                        .call_on(
                            target,
                            ExportSession {
                                session: SessionId::random(),
                                force: false,
                            },
                        )
                        .await
                });
                let (transfer, holder) = peer.close_seen().await;
                finish_call(completion, caller, &mut peer, &owner, &home.forwarders).await;
                let origin = peer.finish(transfer, holder).await;
                assert_eq!(origin.node, node.info().id);
                assert_eq!(origin.by, OriginBy::Owner);
                assert_eq!(origin.scopes, Scope::ALL);
            }
        }
        fleet.stop().await;
    })
    .await
    .unwrap();
}
