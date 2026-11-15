//! Real current-parent and actor evidence, including committed detach before publication.
use super::{Fixture, Found, LaunchOptions, SAFETY_NET, ready};
use vessel_proto::{
    ContentBlock, Creator, NodeId, SessionInput, SessionRef,
    session_archive::{ArchiveChildSession, ChildArchiveOutcome, ChildArchiveSkip},
};

async fn child(fixture: &Fixture) -> ArchiveChildSession {
    let parent = SessionRef {
        node: NodeId::random(),
        session: vessel_proto::SessionId::random(),
    };
    let info = fixture
        .sessions
        .create(
            fixture.structured(),
            LaunchOptions {
                created_by: Creator::Agent {
                    node: parent.node,
                    session: parent.session,
                },
                ..LaunchOptions::default()
            },
            None,
        )
        .await
        .unwrap();
    let Found::Live { handle, .. } = fixture.sessions.find(info.id).await.unwrap() else {
        panic!("child missing");
    };
    let _running = ready(&handle).await.unwrap();
    ArchiveChildSession {
        parent,
        child: SessionRef {
            node: NodeId::random(),
            session: info.id,
        },
    }
}

#[tokio::test]
async fn attached_idle_child_archives_once_with_the_committed_node_timestamp() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(SAFETY_NET, async move {
        let request = child(&fixture).await;
        let outcome = fixture
            .sessions
            .archive_child_with(request, |info| std::future::ready(Ok(info)))
            .await
            .unwrap();
        let ChildArchiveOutcome::Archived { child, archived_at } = outcome else {
            panic!("not archived: {outcome:?}");
        };
        assert_eq!(child, request.child);
        assert_eq!(
            fixture
                .store
                .session(child.session)
                .await
                .unwrap()
                .unwrap()
                .marks
                .archived_at,
            Some(archived_at)
        );
        assert!(matches!(
            fixture
                .sessions
                .archive_child_with(request, |info| std::future::ready(Ok(info)))
                .await
                .unwrap(),
            ChildArchiveOutcome::Skipped {
                reason: ChildArchiveSkip::AlreadyArchived,
                ..
            }
        ));
        fixture.settle().await;
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn current_parent_compares_the_full_remote_reference_without_stopping_another_child() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(SAFETY_NET, async move {
        let mut request = child(&fixture).await;
        request.parent.node = NodeId::random();
        let outcome = fixture
            .sessions
            .archive_child_with(request, |_| async { panic!("ineligible cleanup ran") })
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            ChildArchiveOutcome::Skipped {
                reason: ChildArchiveSkip::ParentChanged,
                ..
            }
        ));
        assert!(
            fixture
                .sessions
                .get(request.child.session)
                .await
                .unwrap()
                .status
                .is_live()
        );
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn committed_detach_beats_an_older_attached_live_actor_snapshot() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(SAFETY_NET, async move {
        let request = child(&fixture).await;
        let old = fixture.sessions.get(request.child.session).await.unwrap();
        assert_eq!(old.parentage().unwrap().parent(), Some(request.parent));
        // Exercise the actual commit/publication gap under the same lock W2
        // holds: the actor still reports its old initial parent metadata.
        let admission = fixture.sessions.marking.lock().await;
        let _detached = fixture
            .store
            .detach(request.child.session, chrono::Utc::now())
            .await
            .unwrap();
        assert_eq!(
            fixture
                .sessions
                .get(request.child.session)
                .await
                .unwrap()
                .parentage()
                .unwrap()
                .parent(),
            Some(request.parent)
        );
        drop(admission);
        let outcome = fixture
            .sessions
            .archive_child_with(request, |_| async { panic!("detached child cleanup ran") })
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            ChildArchiveOutcome::Skipped {
                reason: ChildArchiveSkip::ParentChanged,
                ..
            }
        ));
        let current = fixture.sessions.get(request.child.session).await.unwrap();
        assert!(current.status.is_live());
        assert!(current.marks.archived_at.is_none());
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_running_turn_is_skipped_without_issuing_a_forced_stop() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(SAFETY_NET, async move {
        let request = child(&fixture).await;
        let Found::Live { handle, .. } =
            fixture.sessions.find(request.child.session).await.unwrap()
        else {
            panic!("child missing");
        };
        handle
            .send(SessionInput::Prompt {
                content: vec![ContentBlock::Text {
                    text: "wait".to_owned(),
                }],
            })
            .await
            .unwrap();
        let outcome = fixture
            .sessions
            .archive_child_with(request, |_| async { panic!("busy child cleanup ran") })
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            ChildArchiveOutcome::Skipped {
                reason: ChildArchiveSkip::NotFinished,
                ..
            }
        ));
        assert!(!handle.has_ended());
        assert!(handle.info().marks.archived_at.is_none());
        fixture.stop().await;
    })
    .await
    .unwrap();
}
