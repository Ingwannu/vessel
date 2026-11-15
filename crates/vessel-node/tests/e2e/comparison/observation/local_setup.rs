//! The real shared wait quota also removes a local setup read's wait.
use super::*;
use vessel_proto::Method;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exhausted_local_waits_leave_one_setup_continuation_without_reissuing() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let owner = fixture.connect(&node).await;
        let held = readiness::waits(&owner, &fixture, &owner, node.info().id).await;
        let (folder, project) = crate::tidying::project(&owner).await;
        let fifo = folder.path().join("release");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let mut config = toml_edit::DocumentMut::new();
        config["worktree"]["setup"] = toml_edit::value(format!("cat '{}'", fifo.display()));
        std::fs::write(folder.path().join("vessel.toml"), config.to_string()).unwrap();
        commit(folder.path(), "vessel.toml");
        approve(&owner, project.id, true).await;
        let bridge = Bridge::open(&fixture, &node).await;
        let job = bridge
            .client
            .comparison(node.info().id, plan(project.id, &["fake-small"]));
        let checkpoint = step("local setup with exhausted wait shares", job.prepare())
            .await
            .unwrap();
        assert!(matches!(
            checkpoint.targets[0].status,
            TargetStatus::WaitingSetup
        ));
        let worktree = checkpoint.targets[0].resources.worktree;
        let reads = bridge.count(Method::SetupOutput).await;
        let unchanged = job
            .continue_target(checkpoint.targets[0].index)
            .await
            .unwrap();
        assert_eq!(unchanged.targets[0].resources.worktree, worktree);
        assert_eq!(bridge.count(Method::SetupOutput).await, reads);
        assert_eq!(bridge.count(Method::CreateWorktree).await, 1);
        assert_eq!(bridge.count(Method::CreateSession).await, 0);
        let _cancelled = job.cancel().await.unwrap();
        bridge.stop().await;
        for wait in held {
            wait.abort();
        }
        node.stop().await.unwrap();
    })
    .await;
}
