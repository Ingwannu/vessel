//! Ordinary authorization remains authoritative; no permission-probing mutations.
use super::*;
use crate::support::within;
use vessel_client::comparison::{Phase, TargetStatus};
use vessel_proto::{
    ErrorCode, Scope,
    workspace::{ListWorktrees, WorktreeRefusal},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_scope_callers_leave_zero_worktrees_at_the_existing_refusal_phase() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = crate::workspace::lookup(&fixture);
                crate::secure::advertise_placeholder_route(config);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        for (scopes, phase) in [
            (vec![Scope::Read], Phase::WorktreeCreate),
            (vec![Scope::Operate], Phase::Preflight),
        ] {
            let client = crate::side_questions::access::device(&owner, &node, scopes).await;
            let job = client.comparison(node.info().id, plan(project.id, &["fake-small"]));
            let snapshot = job.prepare().await.unwrap();
            assert!(
                matches!(&snapshot.targets[0].status, TargetStatus::Failed(failure)
                if failure.phase == phase && failure.code == Some(ErrorCode::Forbidden)),
                "{snapshot:?}"
            );
            assert!(snapshot.targets[0].resources.worktree.is_none());
            assert!(snapshot.targets[0].resources.session.is_none());
            assert!(
                owner
                    .call(ListWorktrees {
                        project: Some(project.id)
                    })
                    .await
                    .unwrap()
                    .worktrees
                    .is_empty()
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn comparison_preserves_the_agent_git_guard_and_the_normal_worktree_limit() {
    let fixture = Fixture::new();
    let mcp = crate::agents::vessel_mcp(&fixture);
    within(async move {
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = crate::workspace::lookup(&fixture);
                config.vessel_mcp = Some(mcp);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&owner).await;
        let mut parent = crate::agents::structured(&owner, &fixture).await;
        let (_, socket, credential) = crate::agents::credential(&owner, &mut parent).await;
        let agent = crate::agents::as_agent(&socket, &credential).await;
        for _ in 0..8 {
            let _worktree = crate::workspace::new_worktree(&agent, project.id).await;
        }
        let before = owner
            .call(ListWorktrees {
                project: Some(project.id),
            })
            .await
            .unwrap();
        let limit = agent
            .call(vessel_proto::workspace::CreateWorktree {
                project: project.id,
                branch: vessel_proto::workspace::WorktreeBranch::New {
                    name: None,
                    base: None,
                },
                setup: vessel_proto::workspace::SetupChoice::Run,
            })
            .await
            .unwrap();
        assert!(matches!(
            limit,
            vessel_proto::workspace::CreateWorktreeOutcome::Refused {
                reason: WorktreeRefusal::Limit
            }
        ));
        let job = agent.comparison(node.info().id, plan(project.id, &["fake-small"]));
        let snapshot = job.prepare().await.unwrap();
        assert!(
            matches!(&snapshot.targets[0].status, TargetStatus::Failed(failure)
            if failure.phase == Phase::Preflight && failure.code == Some(ErrorCode::Forbidden)),
            "{snapshot:?}"
        );
        assert!(snapshot.targets[0].resources.worktree.is_none());
        let after = owner
            .call(ListWorktrees {
                project: Some(project.id),
            })
            .await
            .unwrap();
        assert_eq!(before, after);
        node.stop().await.unwrap();
    })
    .await;
}
