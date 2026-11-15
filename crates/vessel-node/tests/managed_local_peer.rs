//! A real same-home replacement must never inherit an old child's RPC authority.

#![cfg(test)]
#![cfg(all(target_os = "linux", feature = "local-node-fixture"))]

#[path = "../src/bin/local_node_fixture/control.rs"]
mod control;
#[path = "managed_local_peer/support.rs"]
mod managed_local_peer_support;

use futures_util::{SinkExt as _, StreamExt as _};
use managed_local_peer_support::{SAFETY_NET, client, recipe, replacement};
use vessel_local_node::{LocalNode, LocalNodeError, LocalStatus, StopPolicy};
use vessel_proto::NodeState;

#[derive(Clone, Copy)]
enum Original {
    ServerStopped,
    Exited,
}

async fn refuses_replacement(original: Original) {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let launch = recipe(root.path(), listener.local_addr().unwrap());
    let (release, held) = std::sync::mpsc::sync_channel(1);
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let starting = tokio::spawn(async move {
        LocalNode::start_observed(launch, move |_logs| {
            // Own the runtime's sole blocking worker before the post-Ready
            // identity read. No product hook or source timing guess is needed.
            drop(tokio::task::spawn_blocking(move || {
                let _delivered = entered.send(());
                let _released = held.recv_timeout(SAFETY_NET);
            }));
        })
        .await
    });
    waiting.await.unwrap();
    let mut fixture = managed_local_peer_support::accept(&listener).await;
    let frame = fixture.next().await.unwrap().unwrap();
    let (process, expected) = match serde_json::from_str(&frame).unwrap() {
        control::Notice::Ready { process, node } => (process, node),
        control::Notice::Stopped { .. } => panic!("stop preceded readiness"),
    };
    fixture
        .send(serde_json::to_string(&control::Command::Stop).unwrap())
        .await
        .unwrap();
    let stopped = fixture.next().await.unwrap().unwrap();
    assert!(
        matches!(serde_json::from_str(&stopped).unwrap(), control::Notice::Stopped { process: found } if process == found)
    );
    let mut successor = replacement(root.path()).await;
    let peer = client(root.path(), expected).await;
    assert_eq!(peer.node_info().await.unwrap().state, NodeState::Serving);
    if matches!(original, Original::Exited) {
        fixture
            .send(serde_json::to_string(&control::Command::Exit).unwrap())
            .await
            .unwrap();
        managed_local_peer_support::zombie(process).await;
    }
    release.send(()).unwrap();
    let result = starting.await.unwrap();
    // Do not let a failed regression leak the original controller owner.
    let refusal = match result {
        Err(error) => error,
        Ok(controller) => panic!("same-home replacement acquired {:?}", controller.status()),
    };
    assert!(
        !matches!(refusal, LocalNodeError::Identity { .. }),
        "replacement deliberately has the same persistent identity"
    );
    assert_eq!(peer.node_info().await.unwrap().state, NodeState::Serving);
    managed_local_peer_support::reaped(process).await;
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
