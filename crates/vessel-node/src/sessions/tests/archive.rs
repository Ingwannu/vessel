//! The actor-admitted archive publishes only a store-owned timestamp.
use super::{Fixture, Found, LaunchOptions, ServiceError, ready};
use vessel_proto::{ContentBlock, SessionInput, session_archive::ArchiveCondition};
use vessel_session::{ArchiveRefusal, SessionError};

#[tokio::test]
async fn idle_archive_is_durable_and_already_archived_is_an_explicit_refusal() {
    let fixture = Fixture::new().await;
    let id = fixture
        .sessions
        .create(fixture.structured(), LaunchOptions::default(), None)
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("live session missing")
    };
    let _running = ready(&handle).await.unwrap();
    let archived = fixture
        .sessions
        .archive_condition(id, ArchiveCondition::IfIdle)
        .await
        .unwrap();
    assert_eq!(archived.id, id);
    assert!(!archived.status.is_live());
    let at = archived.marks.archived_at.unwrap();
    assert_eq!(
        fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .archived_at,
        Some(at)
    );
    let again = fixture
        .sessions
        .archive_condition(id, ArchiveCondition::Force)
        .await;
    assert!(matches!(
        again,
        Err(ServiceError::Session(SessionError::NotArchived(
            ArchiveRefusal::AlreadyArchived
        )))
    ));
    assert_eq!(
        fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .archived_at,
        Some(at)
    );
    fixture.settle().await;
    fixture.stop().await;
}

#[tokio::test]
async fn active_turn_refusal_does_not_write_a_mark_or_stop_the_runtime() {
    let fixture = Fixture::new().await;
    let id = fixture
        .sessions
        .create(fixture.structured(), LaunchOptions::default(), None)
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("live session missing")
    };
    let _running = ready(&handle).await.unwrap();
    handle
        .send(SessionInput::Prompt {
            content: vec![ContentBlock::Text {
                text: "wait".to_owned(),
            }],
        })
        .await
        .unwrap();
    let refused = fixture
        .sessions
        .archive_condition(id, ArchiveCondition::IfIdle)
        .await;
    assert!(matches!(
        refused,
        Err(ServiceError::Session(SessionError::NotArchived(
            ArchiveRefusal::TurnRunning
        )))
    ));
    assert!(handle.info().status.is_live());
    assert!(
        fixture
            .store
            .session(id)
            .await
            .unwrap()
            .unwrap()
            .marks
            .archived_at
            .is_none()
    );
    let ended = fixture
        .sessions
        .archive_condition(id, ArchiveCondition::Force)
        .await
        .unwrap();
    assert!(!ended.status.is_live());
    assert!(ended.marks.archived_at.is_some());
    fixture.settle().await;
    fixture.stop().await;
}

#[tokio::test]
async fn idle_archive_of_an_already_ended_stored_session_preserves_identity() {
    let fixture = Fixture::new().await;
    let id = fixture
        .sessions
        .create(fixture.structured(), LaunchOptions::default(), None)
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("live session missing")
    };
    handle.kill();
    fixture.settle().await;
    assert!(matches!(
        fixture.sessions.find(id).await.unwrap(),
        Found::Stored(_)
    ));
    let archived = fixture
        .sessions
        .archive_condition(id, ArchiveCondition::IfIdle)
        .await
        .unwrap();
    assert_eq!(archived.id, id);
    assert!(archived.marks.archived_at.is_some());
    fixture.stop().await;
}
