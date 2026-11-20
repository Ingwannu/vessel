//! Explicit real-candidate transport diagnostic, not a supported-engine fixture.
//!
//! The exact artifact is run only on the approved host/account. Private fixture
//! storage stays quarantined at a fresh nonce path; no product profile or engine
//! admission is released from a leader/accepted-group cleanup receipt.
use super::actor::{Actor, ActorError};
use super::candidate_network::NetworkFixture;
use super::candidate_policy::PolicyOrigin;
use super::candidate_reuse;
use super::owner::Owner;
use super::*;
use crate::launch::{Plan, ProxyEndpoint};
use crate::preparation::Preparation;
use crate::profile::{Profile, ProfileError};
use crate::targets::{TargetNotice, TargetWatch};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use vessel_proto::WorkspaceId;
use vessel_proto::browser::{BrowserFailure, HumanInput, ViewportSide, ViewportSize};

const APPROVED_FIXTURE_TIMEOUT: Duration = Duration::from_secs(38);

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
    tokio::time::timeout(APPROVED_FIXTURE_TIMEOUT, async {
        let Fixture {
            scope,
            plan,
            root,
            temporary,
            listener,
        } = fixture().await;
        let network = NetworkFixture::start().await;
        let routes = network.routes();
        let address = listener.local_addr().unwrap();
        let endpoint = ProxyEndpoint::try_from(address).unwrap();
        let policy = Arc::new(PolicyOrigin::new(address.port()));
        let proxy = candidate_page::FixtureProxy::start(listener, routes, Arc::clone(&policy));
        let owner = Owner::spawn(plan).unwrap();
        let run = owner.lifetime();
        let (mut owner, readiness) = owner.attest(&scope).await.unwrap();
        let page = if let Some((client, events)) = owner.controller() {
            Some(candidate_page::exercise(client, events, routes, &policy).await)
        } else {
            None
        };
        let actor = Actor::start(owner, network.policy());
        let driven = exercise_actor(&actor).await;
        let revoked = exercise_proxy_loss(&actor, &network).await;
        let stopped = actor.shutdown().await.unwrap();
        let receipt = stopped.receipt.as_ref().unwrap();
        let previous_root = root.clone();
        let previous_temporary = temporary.clone();
        let held = scope
            .run(move || {
                matches!(
                    Profile::prepare(&previous_root, &previous_temporary),
                    Err(ProfileError::Busy)
                )
            })
            .await
            .unwrap();
        readiness.unwrap();
        assert!(receipt.run().same_run(&run));
        assert_eq!(receipt.code(), Some(0));
        assert!(!run.is_live());
        assert!(held);
        page.unwrap().unwrap().verify();
        driven.unwrap();
        revoked.unwrap();
        candidate_reuse::relaunch(&scope, root.clone(), temporary, endpoint, stopped).await;
        proxy.finish().await;
        scope
            .run(move || {
                let destination = root
                    .join("home/Downloads")
                    .join(candidate_page::DOWNLOAD_NAME);
                assert!(!destination.exists());
                assert!(!destination.with_extension("txt.crdownload").exists());
            })
            .await
            .unwrap();
        network.finish().await;
        scope.finish().await;
    })
    .await
    .unwrap();
}

async fn exercise_actor(actor: &Actor) -> Result<(), ActorError> {
    let workspace = WorkspaceId::random();
    let size = ViewportSize {
        width: ViewportSide::try_from(390).unwrap(),
        height: ViewportSide::try_from(844).unwrap(),
    };
    let address = "http://fixture.test/".parse().unwrap();
    let id = actor.open(workspace, address, size).await?;
    assert!(matches!(
        actor.watch(WorkspaceId::random(), id.clone()).await,
        Err(ActorError::Target(crate::targets::TargetError::Missing))
    ));
    let mut watched = actor.watch(workspace, id.clone()).await?;
    let frame = watched
        .frames
        .wait_for(Option::is_some)
        .await
        .unwrap()
        .clone()
        .unwrap();
    assert_eq!(frame.image().size().width().get(), size.width.get());
    assert!(frame.revision() >= watched.initial.revision);
    actor
        .input(
            workspace,
            id.clone(),
            frame.revision(),
            HumanInput::ReleaseAll,
        )
        .await?;
    actor
        .navigate(
            workspace,
            id.clone(),
            "http://fixture.test/other".parse().unwrap(),
        )
        .await?;
    assert!(matches!(
        actor
            .input(
                workspace,
                id.clone(),
                frame.revision(),
                HumanInput::ReleaseAll
            )
            .await,
        Err(ActorError::Page(crate::page::PageError::StaleFrame))
    ));
    actor.close(workspace, id).await?;
    assert!(watched.frames.has_changed().is_err());
    assert_eq!(end_reason(&mut watched).await, BrowserFailure::Closed);
    Ok(())
}

async fn exercise_proxy_loss(actor: &Actor, network: &NetworkFixture) -> Result<(), ActorError> {
    let workspace = WorkspaceId::random();
    let side = ViewportSide::try_from(390).unwrap();
    let viewport = ViewportSize { width: side, height: side };
    let original = "http://fixture.test/".parse().unwrap();
    let tab = actor.open(workspace, original, viewport).await?;
    let mut watched = actor.watch(workspace, tab).await?;
    network.stop_policy();
    assert_eq!(end_reason(&mut watched).await, BrowserFailure::NoEngine);
    assert!(watched.frames.has_changed().is_err());
    Ok(())
}

async fn end_reason(watched: &mut TargetWatch) -> BrowserFailure {
    loop {
        if let TargetNotice::Ended(reason) = watched.notices.recv().await.unwrap() {
            return reason;
        }
    }
}
