//! A rejected actual peer observes EOF, never the persisted authentication token.

use super::managed_local_peer_support::{self as support, HeldLaunch, SAFETY_NET};
use tokio::io::AsyncReadExt as _;

async fn refuses_before_credential() {
    let root = tempfile::tempdir().unwrap();
    let HeldLaunch {
        starting,
        release,
        fixture,
        process,
        expected,
    } = support::held_launch(root.path()).await;
    let directory = vessel_store::DataDir::new(root.path().join("node"));
    let endpoint = directory.local_endpoint(expected);
    // The original server has stopped, but its direct child is still retained.
    // Replace only this isolated test endpoint with a listener in another PID.
    match std::fs::remove_file(&endpoint) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("cannot replace isolated endpoint: {error}"),
    }
    let wrong_peer = tokio::net::UnixListener::bind(endpoint).unwrap();
    release.send(()).unwrap();
    let (mut connection, _) = support::step("wrong peer connection", wrong_peer.accept())
        .await
        .unwrap();
    let mut first = [0_u8; 1];
    assert_eq!(
        support::step(
            "wrong peer zero credential EOF",
            connection.read(&mut first)
        )
        .await
        .unwrap(),
        0,
        "an unqualified peer must receive zero credential or Hello bytes"
    );
    let refusal = support::step("failed startup after original checked cleanup", starting)
        .await
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        refusal,
        vessel_local_node::LocalNodeError::Launch(vessel_local_node::LaunchError::PeerUnverified)
    ));
    support::reaped(process);
    drop(fixture);
}

#[test]
fn actual_wrong_peer_receives_zero_credential_bytes() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
        .block_on(async {
            vessel_testkit::within(SAFETY_NET, refuses_before_credential())
                .await
                .unwrap();
        });
}
