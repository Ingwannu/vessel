//! Explicit real-candidate transport diagnostic, not a supported-engine fixture.
//!
//! The exact artifact is run only on the approved host/account. Private fixture
//! storage stays quarantined at a fresh nonce path; no product profile or engine
//! admission is released from a leader/accepted-group cleanup receipt.
use super::*;
use crate::cdp::{requests, transport};
use crate::launch::{Plan, ProxyEndpoint};
use crate::preparation::Preparation;
use crate::profile::{Profile, ProfileError};
use std::path::PathBuf;
use tokio::net::TcpListener;
use vessel_proc::{Child, OwnedGroupExit, Signal};

struct Fixture {
    scope: Preparation,
    plan: Plan,
    root: PathBuf,
    temporary: PathBuf,
    _proxy: TcpListener,
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
        _proxy: proxy,
    }
}

async fn stop(client: &Client, child: &mut Child) -> OwnedGroupExit {
    let close = client.execute(None, requests::CloseBrowser {}).await;
    let driver = client.shutdown().await;
    if close.is_err() {
        child.signal(Signal::Kill).unwrap();
    }
    let observed = tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait_checked()).await;
    let receipt = match observed {
        Ok(receipt) => receipt.unwrap(),
        Err(_deadline) => {
            child.signal(Signal::Kill).unwrap();
            let _receipt = tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait_checked())
                .await
                .unwrap()
                .unwrap();
            panic!("candidate did not stop within its owned shutdown deadline");
        }
    };
    assert!(close.is_ok(), "owned candidate close was not acknowledged");
    driver.unwrap();
    receipt
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
            _proxy,
        } = fixture().await;
        let proxy = super::candidate_page::FixtureProxy::start(_proxy);
        let spawned = plan.spawn_for_diagnostic().unwrap();
        let run = spawned.child.lifetime();
        let (client, _events) = Client::start(transport::Pipe::new(spawned.stdout, spawned.stdin));
        let mut stderr = spawned.stderr;
        let stderr = tokio::spawn(async move {
            let mut sink = tokio::io::sink();
            tokio::io::copy(&mut stderr, &mut sink).await
        });
        let version = client.execute(None, requests::Version {}).await;
        let (plan, mut child, identity) = scope
            .run(move || {
                let child = spawned.child;
                let identity = plan.verify_started(&child);
                (plan, child, identity)
            })
            .await
            .unwrap();
        let page = if identity.is_ok() && version.is_ok() {
            Some(super::candidate_page::exercise(&client).await)
        } else {
            None
        };
        let receipt = stop(&client, &mut child).await;
        let _discarded_bytes = tokio::time::timeout(SHUTDOWN_TIMEOUT, stderr)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        proxy.finish().await;
        let (plan, held) = scope
            .run(move || {
                let held = matches!(Profile::prepare(&root, &temporary), Err(ProfileError::Busy));
                (plan, held)
            })
            .await
            .unwrap();
        scope.finish().await;
        identity.unwrap();
        assert_eq!(version.unwrap().product.as_str(), "Chrome/149.0.7827.55");
        assert!(receipt.run().same_run(&run));
        assert_eq!(receipt.code(), Some(0));
        assert!(!run.is_live());
        assert!(held);
        page.unwrap().unwrap().verify();
        // No Profile::release or product capacity reuse is certified here.
        drop(plan);
    })
    .await
    .unwrap();
}
