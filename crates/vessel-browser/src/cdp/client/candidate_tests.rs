//! Explicit real-candidate transport diagnostic, not a supported-engine fixture.
//!
//! The exact artifact is run only on the approved host/account. Private fixture
//! storage stays quarantined at a fresh nonce path; no product profile or engine
//! admission is released from a leader/accepted-group cleanup receipt.
use super::candidate_network::NetworkFixture;
use super::owner::Owner;
use super::*;
use crate::launch::{Plan, ProxyEndpoint};
use crate::preparation::Preparation;
use crate::profile::{Profile, ProfileError};
use std::path::PathBuf;
use tokio::net::TcpListener;

struct Fixture {
    scope: Preparation,
    plan: Plan,
    root: PathBuf,
    temporary: PathBuf,
    listener: TcpListener,
}

async fn fixture() -> Fixture {
    assert!(!nix::unistd::geteuid().is_root());
    let scope = Preparation::for_profile(CancellationToken::new());
    // Retain diagnostic files even on failure. They are not reusable storage.
    let parent = scope
        .run(|| {
            tempfile::Builder::new()
                .prefix("vp-")
                .tempdir()
                .unwrap()
                .keep()
        })
        .await
        .unwrap();
    let root = parent.join("profile");
    let temporary = parent.join("tmp");
    let proxy = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let endpoint = ProxyEndpoint::try_from(proxy.local_addr().unwrap()).unwrap();
    let profile_root = root.clone();
    let profile_temp = temporary.clone();
    let plan = scope
        .run(move || {
            let profile = Profile::prepare(&profile_root, &profile_temp).unwrap();
            Plan::inspect(profile, endpoint)
        })
        .await
        .unwrap()
        .unwrap();
    Fixture {
        scope,
        plan,
        root,
        temporary,
        listener: proxy,
    }
}

#[tokio::test]
#[ignore = "requires the exact exported artifact, approved CFT and non-root runtime account"]
async fn approved_candidate_uses_real_rust_driver_and_private_child_pipe() {
    tokio::time::timeout(pipe_tests::FIXTURE_TIMEOUT, async {
        let Fixture {
            scope,
            plan,
            root,
            temporary,
            listener,
        } = fixture().await;
        let network = NetworkFixture::start().await;
        let routes = network.routes();
        let proxy = candidate_page::FixtureProxy::start(listener, routes);
        let owner = Owner::spawn(plan).unwrap();
        let run = owner.lifetime();
        let (mut owner, readiness) = owner.attest(&scope).await.unwrap();
        let page = if let Some((client, events)) = owner.controller() {
            Some(candidate_page::exercise(client, events, routes).await)
        } else {
            None
        };
        let receipt = owner.close().await.unwrap();
        proxy.finish().await;
        network.finish().await;
        let held = scope
            .run(move || matches!(Profile::prepare(&root, &temporary), Err(ProfileError::Busy)))
            .await
            .unwrap();
        scope.finish().await;
        readiness.unwrap();
        assert!(receipt.run().same_run(&run));
        assert_eq!(receipt.code(), Some(0));
        assert!(!run.is_live());
        assert!(held);
        page.unwrap().unwrap().verify();
        // No Profile::release or product capacity reuse is certified here.
        drop(owner);
    })
    .await
    .unwrap();
}
