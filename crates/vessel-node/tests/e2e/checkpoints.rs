//! Real node hooks through fake ACP; storage is reopened only after node shutdown.
use crate::support::{Fixture, agent_of, read_until, status_of, structured_spec, within};
use crate::workspace::{lookup, new_worktree, repository};
use vessel_proto::{
    AgentEvent, ContentBlock, Delivery, EventSeq, SessionStatus,
    checkpoint::{Boundary, CheckpointState},
    workspace::{AddProject, SessionPlace},
};
use vessel_store::{Store, StoreConfig};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "pending FILE_CHECKPOINTS assignment; run this actual bootstrap fixture after activation"]
async fn checkpoint_node_turn_hooks_capture_real_managed_files_and_survive_restart() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let client = fixture.connect(&node).await;
        let project_root = tempfile::tempdir().unwrap();
        repository(project_root.path());
        let project = client
            .call(AddProject {
                path: project_root.path().to_path_buf(),
                name: None,
            })
            .await
            .unwrap();
        let worktree = new_worktree(&client, project.id).await;
        let mut spec = structured_spec(&fixture, None);
        spec.cwd = None;
        spec.place = Some(Box::new(SessionPlace::Worktree {
            worktree: worktree.id,
        }));
        let session = client.create_session(spec).await.unwrap().id;
        let mut stream = client.follow(session, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        client
            .prompt(
                session,
                vec![ContentBlock::Text {
                    text: "checkpoint first".to_owned(),
                }],
                Delivery::Queue,
            )
            .await
            .unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        let first_ended = seen
            .iter()
            .find(|record| matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. })))
            .unwrap()
            .seq;
        // A second delivery cannot pass the first after-turn hook. No timing poll
        // or modelled writer acknowledgement substitutes for that real barrier.
        client
            .prompt(
                session,
                vec![ContentBlock::Text {
                    text: "checkpoint second".to_owned(),
                }],
                Delivery::Queue,
            )
            .await
            .unwrap();
        read_until(&mut stream, &mut seen, |record| {
            record.seq > first_ended
                && matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        node.stop().await.unwrap();
        let store = Store::open(&fixture.data_dir(), StoreConfig::default())
            .await
            .unwrap()
            .store;
        let page = store.checkpoint_page(session, None).await.unwrap();
        let before = page
            .marks
            .iter()
            .filter(|mark| {
                matches!(mark.boundary, Boundary::BeforeTurn { .. })
                    && matches!(mark.state, CheckpointState::Ready { .. })
            })
            .count();
        assert_eq!(
            before, 2,
            "both delivered prompts keep their own actual before image: {page:?}"
        );
        assert!(
            page.marks.iter().any(|mark| mark.boundary
                == Boundary::AfterTurn { ended: first_ended }
                && matches!(mark.state, CheckpointState::Ready { .. })),
            "first turn has its after image: {page:?}"
        );
        for mark in page.marks {
            if let CheckpointState::Ready { checkpoint } = mark.state {
                let selected = store
                    .checkpoint_selected(session, checkpoint)
                    .await
                    .unwrap();
                assert_eq!(selected.key.worktree, worktree.id);
                assert!(
                    selected.snapshot.image().entries().iter().any(|entry| entry
                        .path
                        .relative()
                        .as_str()
                        == "README.md")
                );
            }
        }
        store.close().await;
    })
    .await;
}
