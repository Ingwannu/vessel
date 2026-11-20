//! Exhaust the real local wait quota, and compare it with real delegated reads.
use super::*;
use futures_util::{StreamExt as _, stream::FuturesUnordered};
use vessel_client::comparison::JobStatus;
use vessel_proto::{Method, ReadEvents};

pub(super) async fn waits(
    owner: &NodeClient,
    fixture: &Fixture,
    caller: &NodeClient,
    destination: vessel_proto::NodeId,
) -> FuturesUnordered<tokio::task::JoinHandle<()>> {
    let (session, mut stream, mut seen) = crate::attachments::running(owner, fixture, None).await;
    let _sent = owner
        .prompt(
            session,
            vec![vessel_proto::ContentBlock::Text {
                text: "plain".to_owned(),
            }],
            vessel_proto::Delivery::Start,
        )
        .await
        .unwrap();
    read_until(&mut stream, &mut seen, |record| {
        matches!(
            &record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        )
    })
    .await;
    let cursor = owner.session(session).await.unwrap().next_seq;
    let mut waits = FuturesUnordered::new();
    for _ in 0..9 {
        let client = caller.clone();
        waits.push(tokio::spawn(async move {
            let page = client
                .call_on(
                    destination,
                    ReadEvents {
                        session,
                        from: cursor,
                        limit: 1,
                        wait: Some(60),
                    },
                )
                .await
                .unwrap();
            assert!(page.records.is_empty());
        }));
    }
    // The ninth empty answer proves the eight-share guard is occupied;
    // no sleeps or guessed task-admission delays are used.
    step("exhaust real waiting-read share", waits.next())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(waits.len(), 8);
    waits
}
fn profile(fixture: &Fixture, hanging: bool) {
    let env = if hanging {
        "env = { FAKE_ACP_HANG = \"initialize\" }\n"
    } else {
        ""
    };
    std::fs::write(
        fixture.data_dir().profiles_path(),
        format!("[profiles.ready-gate]\nharness = \"claude\"\n{env}"),
    )
    .unwrap();
}
async fn readiness(
    caller: &Fixture,
    caller_node: &vessel_node::Node,
    target: &Fixture,
    owner: &NodeClient,
    destination: vessel_proto::NodeId,
) {
    profile(target, false);
    let _profiles = owner.reload_profiles().await.unwrap();
    let (_folder, project) = crate::tidying::project(owner).await;
    let bridge = Bridge::open(caller, caller_node).await;
    let mut selection = Selection::new(
        HarnessKind::ClaudeCode.into(),
        Some("ready-gate".parse().unwrap()),
        Some(vessel_proto::ModelId::new("fake-small")),
        Vec::new(),
    )
    .unwrap();
    selection.options.clear();
    let plan = ComparisonPlan::new(
        project.id,
        WorktreeBase::Ref {
            name: "main".parse().unwrap(),
        },
        vec![selection],
    )
    .unwrap();
    let job = bridge.client.comparison(destination, plan);
    let arrival = bridge.hold(Method::CreateWorktree).await;
    let preparation = job.prepare();
    tokio::pin!(preparation);
    tokio::select! {
        result = &mut preparation => panic!("held creation completed {result:?}"),
        held = arrival => held.unwrap(),
    }
    // The normal probe has finished; only the actual session is now gated.
    // This changes a fake profile, not the waiting guard or a product RPC.
    profile(target, true);
    let _profiles = owner.reload_profiles().await.unwrap();
    bridge.release().await;
    let prepared = step("readiness without an available long poll", preparation)
        .await
        .unwrap();
    assert!(
        matches!(prepared.targets[0].status, TargetStatus::WaitingReady),
        "{prepared:?}"
    );
    let session = prepared.targets[0].resources.session.unwrap();
    let reads = bridge.count(Method::ReadEvents).await;
    let unchanged = job
        .continue_target(prepared.targets[0].index)
        .await
        .unwrap();
    assert_eq!(unchanged.targets[0].resources.session, Some(session));
    assert_eq!(bridge.count(Method::ReadEvents).await, reads);
    assert_eq!(bridge.count(Method::CreateSession).await, 1);
    assert_eq!(job.cancel().await.unwrap().status, JobStatus::Cancelled);
    bridge.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exhausted_local_waits_do_not_busy_spin_readiness() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let waits = waits(&owner, &fixture, &owner, node.info().id).await;
        readiness(&fixture, &node, &fixture, &owner, node.info().id).await;
        for wait in waits {
            wait.abort();
        }
        node.stop().await.unwrap();
    })
    .await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delegated_member_readiness_does_not_busy_spin_or_recreate() {
    within(async {
        let relay = crate::secure::Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let member = crate::across::member_custom(
            &relay,
            &home,
            "ready member",
            vec![Scope::Sync],
            vessel_node::fleet::DEFAULT_GRANT.to_vec(),
            with_git,
        )
        .await;
        let held_waits = waits(&member.owner, &member.fixture, &home.owner, member.id()).await;
        readiness(
            &home.fixture,
            &home.node,
            &member.fixture,
            &member.owner,
            member.id(),
        )
        .await;
        for wait in held_waits {
            wait.abort();
        }
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
