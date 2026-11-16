//! A queued real root operation never follows a replaced registry path.
use super::*;

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);

#[tokio::test]
async fn queued_mutation_revalidates_after_the_project_reservation() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("write-before", None).await;
        let replacement = fixture.repo("write-after", None);
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let held = fixture.workspaces.locked_git(root).await.unwrap();
        let registered = fixture.workspaces.git_writers.waiting.notified();
        tokio::pin!(registered);
        let _enabled = registered.as_mut().enable();
        let workspaces = Arc::clone(&fixture.workspaces);
        let queued = tokio::spawn(async move { workspaces.locked_git(root).await });
        registered.await;
        {
            let mut state = fixture.workspaces.state();
            state.projects.get_mut(&project.id).unwrap().info.root = replacement;
        }
        drop(held);
        assert!(matches!(
            queued.await.unwrap(),
            Err(WorkspaceError::TargetChanged)
        ));
        fixture.stop.cancel();
        assert!(fixture.tasks.close());
        fixture.tasks.wait().await;
    })
    .await
    .expect("registered-root operation should settle after releasing its real queue barrier");
}
