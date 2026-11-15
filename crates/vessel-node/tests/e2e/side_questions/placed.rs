//! Inherited places resolve through the normal project/worktree boundary.
//! A stored parent has both place and resolved cwd; a new client spec must
//! leave cwd absent whenever place chooses it (ADR 0045).

use super::*;
use vessel_proto::workspace::SessionPlace;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn side_open_and_reset_preserve_project_and_worktree_locations() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = crate::workspace::lookup(&fixture))
            .await;
        let client = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&client).await;
        std::fs::create_dir(project.root.join("nested")).unwrap();
        let worktree = crate::workspace::new_worktree(&client, project.id).await;
        for place in [
            SessionPlace::Project {
                project: project.id,
                dir: Some("nested".parse().unwrap()),
            },
            SessionPlace::Worktree {
                worktree: worktree.id,
            },
        ] {
            let mut spec = structured_spec(&fixture, None).with_place(place);
            spec.cwd = None;
            let parent = client.create_session(spec).await.unwrap();
            let _parent_stream = ready(&client, parent.id).await;
            let before = read_all(&client, parent.id).await;
            let side = client.open_side_question(parent.id).await.unwrap();
            let _side_stream = ready(&client, side.id).await;
            assert_eq!(side.spec.place, parent.spec.place);
            assert_eq!(side.spec.cwd, parent.spec.cwd);
            assert_eq!(side.workspace, parent.workspace);
            let reset = client
                .reset_side_question(parent.id, side.id)
                .await
                .unwrap();
            let _reset_stream = ready(&client, reset.id).await;
            assert_eq!(reset.spec.place, parent.spec.place);
            assert_eq!(reset.spec.cwd, parent.spec.cwd);
            assert_eq!(reset.workspace, parent.workspace);
            assert_eq!(read_all(&client, parent.id).await, before);
            assert!(
                worktree.path.is_dir(),
                "a side reset must not remove its parent's worktree"
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}
