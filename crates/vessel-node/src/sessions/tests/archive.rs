//! The actor-admitted archive publishes only a store-owned timestamp.
use super::{Fixture, Found, LaunchOptions, ServiceError, ready};
use std::time::Duration;
use vessel_proto::{ContentBlock, SessionInput, session_archive::ArchiveCondition};
use vessel_session::{ArchiveRefusal, SessionError};

mod child;
mod resume;

const SAFETY_NET: Duration = Duration::from_secs(30);

#[tokio::test]
async fn idle_archive_is_durable_and_already_archived_is_an_explicit_refusal() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
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
        }),
    )
    .await
    .expect("archive workflow timed out");
}

#[tokio::test]
async fn active_turn_refusal_does_not_write_a_mark_or_stop_the_runtime() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
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
        }),
    )
    .await
    .expect("archive workflow timed out");
}

#[tokio::test]
async fn idle_archive_of_an_already_ended_stored_session_preserves_identity() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
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
        }),
    )
    .await
    .expect("archive workflow timed out");
}

#[tokio::test]
async fn legacy_archive_keeps_first_timestamp_while_new_action_refuses_replay() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
            let id = fixture
                .sessions
                .create(fixture.structured(), LaunchOptions::default(), None)
                .await
                .unwrap()
                .id;
            let first = fixture
                .sessions
                .mark(id, vessel_proto::Mark::Archive)
                .await
                .unwrap();
            let second = fixture
                .sessions
                .mark(id, vessel_proto::Mark::Archive)
                .await
                .unwrap();
            assert_eq!(first.marks.archived_at, second.marks.archived_at);
            assert!(!second.status.is_live());
            let refused = fixture
                .sessions
                .archive_condition(id, ArchiveCondition::Force)
                .await;
            assert!(matches!(
                refused,
                Err(ServiceError::Session(SessionError::NotArchived(
                    ArchiveRefusal::AlreadyArchived
                )))
            ));
            fixture.settle().await;
            fixture.stop().await;
        }),
    )
    .await
    .expect("archive workflow timed out");
}

#[tokio::test]
async fn cancelled_rpc_keeps_admitted_cleanup_and_resume_exclusion_until_completion() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
            let id = fixture
                .sessions
                .create(fixture.structured(), LaunchOptions::default(), None)
                .await
                .unwrap()
                .id;
            let (entered, waiting) = tokio::sync::oneshot::channel();
            let (release, released) = tokio::sync::oneshot::channel();
            let (completed, completion) = tokio::sync::oneshot::channel();
            let sessions = std::sync::Arc::clone(&fixture.sessions);
            let request = tokio::spawn(async move {
                sessions
                    .archive_condition_with(id, ArchiveCondition::Force, move |info| async move {
                        assert!(!info.status.is_live());
                        assert!(info.marks.archived_at.is_some());
                        entered.send(()).unwrap();
                        released.await.unwrap();
                        completed.send(info.id).unwrap();
                        Ok(info)
                    })
                    .await
            });
            waiting.await.unwrap();
            assert!(fixture.sessions.marking.try_lock().is_err());
            request.abort();
            assert!(request.await.unwrap_err().is_cancelled());
            assert!(fixture.sessions.marking.try_lock().is_err());
            let mark_sessions = std::sync::Arc::clone(&fixture.sessions);
            let unarchive =
                async move { mark_sessions.mark(id, vessel_proto::Mark::Unarchive).await };
            tokio::pin!(unarchive);
            std::future::poll_fn(|cx| {
                assert!(unarchive.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            release.send(()).unwrap();
            assert_eq!(completion.await.unwrap(), id);
            assert!(unarchive.await.unwrap().marks.archived_at.is_none());
            fixture.settle().await;
            fixture.stop().await;
        }),
    )
    .await
    .expect("archive cleanup timed out");
}

#[tokio::test]
async fn terminal_archive_requires_force_without_claiming_terminal_quietness_is_idle() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async move {
            let id = fixture
                .sessions
                .create(fixture.spec(), LaunchOptions::default(), None)
                .await
                .unwrap()
                .id;
            let refused = fixture
                .sessions
                .archive_condition(id, ArchiveCondition::IfIdle)
                .await;
            assert!(matches!(
                refused,
                Err(ServiceError::Session(SessionError::NotArchived(
                    ArchiveRefusal::Terminal
                )))
            ));
            let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
                panic!("live terminal missing")
            };
            assert!(handle.info().status.is_live());
            assert!(handle.info().marks.archived_at.is_none());
            let ended = fixture
                .sessions
                .archive_condition(id, ArchiveCondition::Force)
                .await
                .unwrap();
            assert!(!ended.status.is_live());
            assert!(ended.marks.archived_at.is_some());
            fixture.settle().await;
            fixture.stop().await;
        }),
    )
    .await
    .expect("terminal archive timed out");
}
