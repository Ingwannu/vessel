//! A stale ended snapshot cannot authorize stopping a newly resumed active run.
use super::*;
use std::{sync::Arc, task::Poll};
use vessel_proto::SessionId;
use vessel_session::Relaunch;

async fn stored(fixture: &Fixture) -> SessionId {
    let id = fixture
        .sessions
        .create(fixture.structured(), LaunchOptions::default(), None)
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("missing created session")
    };
    let _ready = ready(&handle).await.unwrap();
    handle.kill();
    fixture.settle().await;
    assert!(matches!(
        fixture.sessions.find(id).await.unwrap(),
        Found::Stored(_)
    ));
    id
}

#[tokio::test]
async fn stored_archive_retains_resume_publication_exclusion_through_completion() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(SAFETY_NET, async move {
        let id = stored(&fixture).await;
        let ended = fixture.sessions.ended(id).await.unwrap();
        let claim = fixture.sessions.claim(id).unwrap();
        let (entered, waiting) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let sessions = Arc::clone(&fixture.sessions);
        let archive = tokio::spawn(async move {
            sessions.archive_condition_with(id, ArchiveCondition::IfIdle, move |info| async move {
                entered.send(info.marks.archived_at.unwrap()).unwrap();
                released.await.unwrap();
                Ok(info)
            }).await
        });
        let at = waiting.await.unwrap();
        let resume_sessions = Arc::clone(&fixture.sessions);
        let resuming = async move {resume_sessions.start_again(ended, Relaunch::default(), claim).await};
        tokio::pin!(resuming);
        std::future::poll_fn(|cx| {
            assert!(resuming.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        }).await;
        // Observe the real gated runtime, rather than treating one pending poll
        // as proof that resume reached the shared publication admission.
        while fixture.runtime.get(id).is_none() {tokio::task::yield_now().await;}
        assert!(fixture.sessions.marking.try_lock().is_err());
        assert!(matches!(fixture.sessions.find(id).await.unwrap(), Found::Stored(info) if info.marks.archived_at == Some(at)));
        std::future::poll_fn(|cx| {
            assert!(resuming.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        }).await;
        release.send(()).unwrap();
        let archived = archive.await.unwrap().unwrap();
        assert_eq!(archived.id, id);
        assert!(!archived.status.is_live());
        let resumed = resuming.await.unwrap();
        let _ready = ready(&resumed).await.unwrap();
        assert_eq!(resumed.info().marks.archived_at, Some(at));
        resumed.kill();
        fixture.settle().await;
        fixture.stop().await;
    }).await.expect("stored archive/resume admission timed out");
}

#[tokio::test]
async fn idle_archive_refinds_a_resumed_active_run_after_an_ended_snapshot() {
    let fixture = Fixture::new().await;
    vessel_testkit::within(SAFETY_NET, async move {
        let id = stored(&fixture).await;
        let ended = fixture.sessions.ended(id).await.unwrap();
        assert!(!ended.info.status.is_live());
        let claim = fixture.sessions.claim(id).unwrap();
        let resumed = fixture
            .sessions
            .start_again(ended, Relaunch::default(), claim)
            .await
            .unwrap();
        let _ready = ready(&resumed).await.unwrap();
        resumed
            .send(SessionInput::Prompt {
                content: vec![ContentBlock::Text {
                    text: "wait".to_owned(),
                }],
            })
            .await
            .unwrap();
        assert!(matches!(
            fixture
                .sessions
                .archive_condition(id, ArchiveCondition::IfIdle)
                .await,
            Err(ServiceError::Session(SessionError::NotArchived(
                ArchiveRefusal::TurnRunning
            )))
        ));
        assert!(resumed.info().status.is_live());
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
        let _ended = fixture
            .sessions
            .archive_condition(id, ArchiveCondition::Force)
            .await
            .unwrap();
        fixture.settle().await;
        fixture.stop().await;
    })
    .await
    .expect("stale ended snapshot archive timed out");
}
