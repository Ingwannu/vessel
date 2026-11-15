//! Actual negotiated older peers, and one commit pinned before a moving ref.
use super::faults::bridge::Bridge;
use super::*;
use crate::support::within;
use vessel_client::comparison::{FailureKind, Phase};
use vessel_proto::workspace::{ListWorktrees, ReadLog, WorkspaceRoot};
use vessel_proto::{Method, ProtocolVersion};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn negotiated_older_calls_or_added_agents_are_refused_before_any_rpc() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        for (version, harness) in [
            (9, HarnessKind::ClaudeCode.into()),
            (
                10,
                vessel_proto::HarnessRef::Added("goose".parse().unwrap()),
            ),
        ] {
            let bridge = Bridge::at_protocol(&fixture, &node, ProtocolVersion::new(version)).await;
            let selections = vec![Selection::new(harness, None, None, Vec::new()).unwrap()];
            let plan = ComparisonPlan::new(
                ProjectId::random(),
                WorktreeBase::Ref {
                    name: "main".parse().unwrap(),
                },
                selections,
            )
            .unwrap();
            let job = bridge.client.comparison(node.info().id, plan);
            assert_eq!(
                bridge.count(Method::NodeInfo).await,
                0,
                "construction does not send"
            );
            let report = job.prepare().await.unwrap();
            assert!(
                matches!(&report.targets[0].status, TargetStatus::Failed(failure)
                if failure.kind == FailureKind::Protocol && failure.phase == Phase::Preflight)
            );
            assert_eq!(bridge.count(Method::NodeInfo).await, 0);
            assert_eq!(bridge.count(Method::GitLog).await, 0);
            assert_eq!(bridge.count(Method::CreateWorktree).await, 0);
            bridge.stop().await;
        }
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_branch_advancing_between_creates_does_not_change_the_pinned_base() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (folder, project) = crate::tidying::project(&owner).await;
        let request = || ReadLog {
            root: WorkspaceRoot::Project {
                project: project.id,
            },
            from: Some(WorktreeBase::Ref {
                name: "main".parse().unwrap(),
            }),
            skip: 0,
            limit: Some(1),
        };
        let old = owner.call(request()).await.unwrap().commits[0].id.clone();
        let bridge = Bridge::open(&fixture, &node).await;
        let job = bridge.client.comparison(
            node.info().id,
            plan(project.id, &["fake-small", "fake-large"]),
        );
        let arrival = bridge.hold(Method::CreateWorktree).await;
        let preparing = job.prepare();
        tokio::pin!(preparing);
        tokio::select! {
            reply = &mut preparing => panic!("held first create completed {reply:?}"),
            held = arrival => held.unwrap(),
        }
        assert_eq!(
            bridge.count(Method::CreateWorktree).await,
            1,
            "bounded worker admits only the first"
        );
        std::fs::write(folder.path().join("README.md"), "new main\n").unwrap();
        observation::commit(folder.path(), "README.md");
        let new = owner.call(request()).await.unwrap().commits[0].id.clone();
        assert_ne!(old, new);
        bridge.release().await;
        let report = preparing.await.unwrap();
        assert!(
            report
                .targets
                .iter()
                .all(|target| matches!(target.status, TargetStatus::Ready))
        );
        let worktrees = owner
            .call(ListWorktrees {
                project: Some(project.id),
            })
            .await
            .unwrap();
        assert_eq!(worktrees.worktrees.len(), 2);
        assert!(
            worktrees
                .worktrees
                .iter()
                .all(|worktree| worktree.base.as_ref().unwrap().commit == old)
        );
        assert_eq!(bridge.count(Method::GitLog).await, 1);
        assert_eq!(bridge.count(Method::CreateWorktree).await, 2);
        assert_eq!(bridge.count(Method::PromptSession).await, 0);
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_setup_keeps_the_worktree_and_never_creates_or_prompts_a_session() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (folder, project) = crate::tidying::project(&owner).await;
        std::fs::write(
            folder.path().join("vessel.toml"),
            "[worktree]\nsetup = \"exit 7\"\n",
        )
        .unwrap();
        observation::commit(folder.path(), "vessel.toml");
        observation::approve(&owner, project.id, true).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = bridge
            .client
            .comparison(node.info().id, plan(project.id, &["fake-small"]));
        let report = job.prepare().await.unwrap();
        assert!(
            matches!(&report.targets[0].status, TargetStatus::Failed(failure)
            if failure.phase == Phase::Setup && failure.kind == FailureKind::Setup)
        );
        assert!(report.targets[0].resources.worktree.is_some());
        assert!(report.targets[0].resources.session.is_none());
        assert_eq!(bridge.count(Method::CreateSession).await, 0);
        assert_eq!(bridge.count(Method::PromptSession).await, 0);
        let worktrees = owner
            .call(ListWorktrees {
                project: Some(project.id),
            })
            .await
            .unwrap();
        assert!(matches!(
            worktrees.worktrees[0].setup,
            vessel_proto::workspace::SetupState::Failed {
                failure: vessel_proto::workspace::SetupFailure::Exit { code: Some(7), .. },
                ..
            }
        ));
        bridge.stop().await;
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_installed_provider_without_structured_support_stops_shared_preflight() {
    within(async {
        let fixture = Fixture::new();
        drop(vessel_harness::fakes::install(fixture.bin(), HarnessKind::Codex,
            vessel_harness::fakes::Role::Terminal, &vessel_testkit::Fake::Tui.path().unwrap()).unwrap());
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let unavailable = owner.harness_options(HarnessKind::Codex, None).await.unwrap();
        assert!(unavailable.abilities.is_none(), "missing adapter is a successful options response, not availability");
        let selections = vec![Selection::new(HarnessKind::ClaudeCode.into(), None, None, Vec::new()).unwrap(),
            Selection::new(HarnessKind::Codex.into(), None, None, Vec::new()).unwrap()];
        let job = owner.comparison(node.info().id, ComparisonPlan::new(project.id,
            WorktreeBase::Ref { name: "main".parse().unwrap() }, selections).unwrap());
        let report = job.prepare().await.unwrap();
        assert!(report.targets.iter().all(|target| matches!(&target.status,
            TargetStatus::Failed(failure) if failure.phase == Phase::Preflight && failure.kind == FailureKind::Provider)));
        assert!(report.targets.iter().all(|target| target.resources.worktree.is_none() && target.resources.session.is_none()));
        assert!(owner.call(ListWorktrees { project: Some(project.id) }).await.unwrap().worktrees.is_empty());
        assert!(owner.sessions().await.unwrap().is_empty());
        node.stop().await.unwrap();
    }).await;
}
