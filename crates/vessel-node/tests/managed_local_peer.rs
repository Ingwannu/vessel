//! A real same-home replacement must never inherit an old child's RPC authority.

#![cfg(test)]
#![cfg(all(target_os = "linux", feature = "local-node-fixture"))]

#[path = "../src/bin/local_node_fixture/control.rs"]
mod control;
#[path = "managed_local_peer/credential.rs"]
mod credential;
#[path = "managed_local_peer/support.rs"]
mod managed_local_peer_support;

use futures_util::{SinkExt as _, StreamExt as _};
use managed_local_peer_support::{SAFETY_NET, client, replacement};
use vessel_local_node::{LocalNode, LocalNodeError, LocalStatus, StopPolicy};
use vessel_proto::NodeState;

#[derive(Clone, Copy)]
enum Original {
    ServerStopped,
    Exited,
}

async fn refuses_replacement(original: Original) {
    let root = tempfile::tempdir().unwrap();
    let managed_local_peer_support::HeldLaunch {
        starting,
        release,
        mut fixture,
        process,
        expected,
    } = managed_local_peer_support::held_launch(root.path()).await;
    let mut successor = replacement(root.path()).await;
    let peer = client(root.path(), expected).await;
    assert_eq!(peer.node_info().await.unwrap().state, NodeState::Serving);
    if matches!(original, Original::Exited) {
        let mut exited =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child()).unwrap();
        fixture
            .send(serde_json::to_string(&control::Command::Exit).unwrap())
            .await
            .unwrap();
        managed_local_peer_support::zombie(process, &mut exited).await;
    }
    release.send(()).unwrap();
    let result = starting.await.unwrap();
    // Do not let a failed regression leak the original controller owner.
    let refusal = match result {
        Err(error) => error,
        Ok(controller) => panic!("same-home replacement acquired {:?}", controller.status()),
    };
    assert!(
        matches!(
            refusal,
            LocalNodeError::Launch(vessel_local_node::LaunchError::PeerUnverified)
        ),
        "same-identity replacement must fail the actual peer proof before child-exit selection"
    );
    assert_eq!(peer.node_info().await.unwrap().state, NodeState::Serving);
    managed_local_peer_support::reaped(process);
    let mut attached = LocalNode::start(managed_local_peer_support::ordinary_recipe(root.path()))
        .await
        .unwrap();
    assert_eq!(attached.status(), LocalStatus::Attached(expected));
    assert!(matches!(
        attached.stop(StopPolicy::InterruptConfirmed).await,
        Err(LocalNodeError::NotOwned)
    ));
    assert!(matches!(
        attached.restart(StopPolicy::InterruptConfirmed).await,
        Err(LocalNodeError::NotOwned)
    ));
    assert_eq!(peer.node_info().await.unwrap().state, NodeState::Serving);
    peer.stop_node(vessel_proto::StopWhen::WhenIdle)
        .await
        .unwrap();
    let _exit = successor.wait_checked().await.unwrap();
}

fn run(original: Original) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
        .block_on(async {
            vessel_testkit::within(SAFETY_NET, refuses_replacement(original))
                .await
                .unwrap();
        });
}

#[test]
fn live_original_with_stopped_server_cannot_authorize_same_home_replacement() {
    run(Original::ServerStopped);
}

#[test]
fn exited_original_cannot_authorize_same_home_replacement_before_reaping() {
    run(Original::Exited);
}
