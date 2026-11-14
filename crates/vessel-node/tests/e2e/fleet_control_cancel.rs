//! Removing membership must cancel its in-flight grant before reconciliation
//! waits on that grant's lock. This is a leave, not node shutdown.
use crate::fleet::fleet_config;
use crate::secure::Relay;
use crate::support::{Fixture, within};
use std::time::Duration;
use vessel_proto::{FleetRole, HarnessKind, Scope};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn leaving_does_not_wait_for_the_grant_it_needs_to_cancel() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let dialed = std::sync::Arc::new(tokio::sync::Notify::new());
        let seen = std::sync::Arc::clone(&dialed);
        let silent = tokio::spawn(async move {
            let mut sockets = Vec::new();
            for _ in 0..8 {
                let (socket, _) = listener.accept().await.unwrap();
                sockets.push(socket);
                seen.notify_one();
            }
            std::future::pending::<()>().await;
        });
        let fixture = Fixture::new();
        let applied = fixture.cwd().join("member-codex");
        let node = fixture
            .start_with(|config| {
                config.name = Some("leave-pending-grant".to_owned());
                config.public = (0..8)
                    .map(|index| format!("ws://{address}/r{index}").parse().unwrap())
                    .collect();
                config.fleet = fleet_config(HarnessKind::Codex, applied.clone());
            })
            .await;
        let owner = fixture.connect(&node).await;
        let link = home
            .owner
            .invite_with_scopes(None, 600, vec![Scope::Sync])
            .await
            .unwrap()
            .link;
        let _joined = owner
            .join_fleet(link, vessel_node::fleet::DEFAULT_GRANT.to_vec())
            .await
            .unwrap();
        crate::support::step("home begins pairing its grant", dialed.notified()).await;
        tokio::time::timeout(Duration::from_secs(5), owner.leave_fleet())
            .await
            .expect("leave waited for its grant instead of cancelling it")
            .unwrap();
        assert_eq!(
            owner.fleet_status().await.unwrap().role,
            FleetRole::Standalone
        );
        assert!(
            !applied.join("AGENTS.md").exists(),
            "leave admitted a first pull after cancelling its grant"
        );
        node.stop().await.unwrap();
        silent.abort();
        assert!(silent.await.unwrap_err().is_cancelled());
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
