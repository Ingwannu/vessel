//! Real managed worktree + shared writer + private Linux archive, no agent launch.
use super::*;
use vessel_proto::{
    EventRecord, EventSeq, HarnessKind, SessionEvent, SessionInfo, SessionMode, SessionSpec,
    checkpoint::{
        Boundary, CHECKPOINT_CAPTURE_DEADLINE, CaptureGeneration, CaptureId, CaptureKey,
        CheckpointEvent, CheckpointState,
    },
};
use vessel_session::CheckpointAdvance;
use vessel_workspace::checkpoint::{IgnoreDefaults, Storage};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_backend_commits_anchor_before_io_and_waits_for_actual_actor_delivery() {
    for advance in [
        CheckpointAdvance::PromptSent,
        CheckpointAdvance::PromptRefused,
    ] {
        let fixture = Fixture::new().await;
        let project = fixture.project("checkpoint-project", None).await;
        let worktree = fixture.created(project.id).await;
        let session = SessionId::random();
        fixture
            .store
            .create_session(SessionInfo::new(
                session,
                SessionSpec::new(
                    HarnessKind::Codex,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                ),
                chrono::Utc::now(),
            ))
            .await
            .unwrap();
        fixture
            .store
            .append(
                session,
                vec![Arc::new(EventRecord {
                    seq: EventSeq::ZERO,
                    at: chrono::Utc::now(),
                    event: SessionEvent::Checkpoint {
                        event: CheckpointEvent::PromptAccepted,
                    },
                })],
            )
            .await
            .unwrap();
        let key = CaptureKey {
            session,
            capture: CaptureId::random(),
            worktree: worktree.id,
            boundary: Boundary::BeforeTurn {
                prompt: EventSeq::ZERO,
            },
            generation: CaptureGeneration::FIRST,
        };
        let physical = vessel_workspace::physical::Directory::open(&worktree.path)
            .unwrap()
            .identity();
        let (client, owner) = crate::checkpoint::backend::start(
            Arc::clone(&fixture.workspaces),
            fixture.store.clone(),
            IgnoreDefaults::new(fixture.path().join("absent-global-ignore")).unwrap(),
        );
        let accepted = client
            .capture_candidate(
                physical,
                key,
                tokio::time::Instant::now() + CHECKPOINT_CAPTURE_DEADLINE,
            )
            .await
            .unwrap();
        let page = fixture.store.checkpoint_page(session, None).await.unwrap();
        assert!(
            !page
                .marks
                .iter()
                .any(|mark| matches!(mark.state, CheckpointState::Ready { .. }))
        );
        accepted.complete(advance);
        drop(client);
        owner.await.unwrap();
        let page = fixture.store.checkpoint_page(session, None).await.unwrap();
        assert_eq!(page.marks.len(), 1);
        match advance {
            CheckpointAdvance::PromptSent => {
                let CheckpointState::Ready { checkpoint } = page.marks[0].state else {
                    panic!("no ready image");
                };
                let selected = fixture
                    .store
                    .checkpoint_selected(session, checkpoint)
                    .await
                    .unwrap();
                assert_eq!(selected.key, key);
                assert_eq!(selected.snapshot.evidence().physical(), physical);
                assert!(!selected.snapshot.image().entries().is_empty());
                let storage = Storage::open(fixture.data.root()).unwrap();
                let blobs = storage.open_capture(key).unwrap().archive().unwrap();
                drop(blobs);
                fixture
                    .store
                    .append(
                        session,
                        vec![Arc::new(EventRecord {
                            seq: EventSeq::new(1),
                            at: chrono::Utc::now(),
                            event: SessionEvent::Status {
                                status: vessel_proto::SessionStatus::Exited { code: Some(0) },
                            },
                        })],
                    )
                    .await
                    .unwrap();
                fixture.store.delete_session(session).await.unwrap();
                assert_eq!(fixture.store.checkpoint_garbage().await.unwrap().len(), 1);
                crate::checkpoint::gc::test_pass(&fixture.store, &fixture.workspaces)
                    .await
                    .unwrap();
                assert!(fixture.store.checkpoint_garbage().await.unwrap().is_empty());
                assert!(storage.open_capture(key).is_err());
            }
            CheckpointAdvance::PromptRefused => {
                assert!(matches!(
                    page.marks[0].state,
                    CheckpointState::Unavailable { .. }
                ));
                assert!(
                    Storage::open(fixture.data.root())
                        .unwrap()
                        .open_capture(key)
                        .is_err()
                );
            }
            CheckpointAdvance::TurnComplete => unreachable!(),
        }
        fixture.stop.cancel();
        let _closed = fixture.tasks.close();
        fixture.tasks.wait().await;
        fixture.store.close().await;
    }
}
