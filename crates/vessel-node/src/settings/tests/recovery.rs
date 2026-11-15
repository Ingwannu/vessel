//! Cancellation and real commit failures for startup policy, ADR 0059.
use super::*;

/// Safety net for retained write/apply work, never a product timeout.
const WAIT: Duration = Duration::from_secs(20);

fn recovery_change(enabled: bool) -> SavedNodeSettings {
    SavedNodeSettings {
        restart_recovery: Some(vessel_proto::restart::SetRestartRecovery {
            enabled: Some(enabled),
            prompt: None,
        }),
        ..SavedNodeSettings::default()
    }
}
async fn pending(f: &Fixture) -> vessel_proto::SessionId {
    use vessel_proto::restart::{RestartCheckpoint, RestartGeneration};
    use vessel_proto::{
        AgentEvent, EventRecord, EventSeq, HarnessKind, SessionEvent, SessionInfo, SessionMode,
        SessionSpec, SessionStatus,
    };
    let store = &f.settings.engine.store;
    let info = SessionInfo::new(
        vessel_proto::SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::Utc::now(),
    );
    store
        .create_session_with_restart(info.clone(), vessel_store::RestartEligibility::Person)
        .await
        .unwrap();
    assert!(
        store
            .checkpoint_restart(
                info.id,
                vec![Arc::new(EventRecord {
                    seq: EventSeq::ZERO,
                    at: chrono::Utc::now(),
                    event: SessionEvent::Agent {
                        event: AgentEvent::TurnStarted {
                            content: vec![vessel_proto::ContentBlock::Text {
                                text: "working".into(),
                            }],
                        },
                    },
                })],
                RestartCheckpoint::Shutdown {
                    generation: RestartGeneration::ZERO,
                    turn: Some(EventSeq::ZERO),
                    parked: false,
                },
            )
            .await
            .unwrap()
    );
    store
        .append(
            info.id,
            vec![Arc::new(EventRecord {
                seq: EventSeq::new(1),
                at: chrono::Utc::now(),
                event: SessionEvent::Status {
                    status: SessionStatus::Interrupted,
                },
            })],
        )
        .await
        .unwrap();
    assert_eq!(
        store.next_restart().await.unwrap().unwrap().session,
        info.id
    );
    info.id
}
#[tokio::test]
async fn caller_loss_during_off_write_keeps_the_barrier_and_then_consumes_intent() {
    tokio::time::timeout(WAIT, async {
        let f = Fixture::new(None).await;
        assert!(
            f.settings
                .change(recovery_change(true))
                .await
                .unwrap()
                .restart_recovery
                .enabled
        );
        let _id = pending(&f).await;
        let (_, generation) = f.settings.policy().snapshot();
        let (entered, release) = f.settings.engine.gates.arm_write();
        let settings = Arc::clone(&f.settings);
        let caller = tokio::spawn(async move { settings.change(recovery_change(false)).await });
        entered.await.unwrap();
        assert!(!f.settings.policy().check(generation).unwrap());
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        release.send(()).unwrap();
        f.settings.drain().await;
        assert!(!f.settings.current().await.restart_recovery.enabled);
        assert!(
            !f.data
                .read_settings_record()
                .unwrap()
                .unwrap()
                .restart_recovery
                .unwrap()
                .effective()
                .enabled
        );
        assert!(
            f.settings
                .engine
                .store
                .next_restart()
                .await
                .unwrap()
                .is_none()
        );
        assert!(f.settings.policy().check(generation).is_err());
        f.finish().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn lost_reply_after_saved_off_cannot_let_queued_on_overtake_cleanup() {
    tokio::time::timeout(WAIT, async {
        let f = Fixture::new(None).await;
        assert!(
            f.settings
                .change(recovery_change(true))
                .await
                .unwrap()
                .restart_recovery
                .enabled
        );
        let _id = pending(&f).await;
        let (_, generation) = f.settings.policy().snapshot();
        let (entered, release) = f.settings.engine.gates.arm_cleanup();
        let settings = Arc::clone(&f.settings);
        let caller = tokio::spawn(async move { settings.change(recovery_change(false)).await });
        entered.await.unwrap();
        assert!(
            !f.data
                .read_settings_record()
                .unwrap()
                .unwrap()
                .restart_recovery
                .unwrap()
                .effective()
                .enabled
        );
        assert!(!f.settings.policy().snapshot().0.enabled);
        assert!(
            f.settings
                .engine
                .store
                .next_restart()
                .await
                .unwrap()
                .is_some()
        );
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        let (reply, on) = oneshot::channel();
        f.settings
            .changes
            .try_send(Change {
                request: recovery_change(true),
                reply,
            })
            .unwrap();
        release.send(()).unwrap();
        assert!(on.await.unwrap().unwrap().restart_recovery.enabled);
        assert!(
            f.settings
                .engine
                .store
                .next_restart()
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            f.settings.policy().check(generation).is_err(),
            "later On cannot restore the old generation"
        );
        f.finish().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn a_real_cleanup_failure_keeps_off_and_blocks_later_on_until_the_obligation_is_paid() {
    tokio::time::timeout(WAIT, async {
        let f=Fixture::new(None).await;
        assert!(f.settings.change(recovery_change(true)).await.unwrap().restart_recovery.enabled); let _id=pending(&f).await;
        let database=rusqlite::Connection::open(f.data.database_path()).unwrap();
        database.execute_batch("CREATE TRIGGER fail_recovery_cleanup BEFORE UPDATE ON restart_control WHEN NEW.enabled=0 BEGIN SELECT RAISE(ABORT,'test cleanup failure'); END").unwrap();
        assert!(matches!(f.settings.change(recovery_change(false)).await,Err(SettingsError::Cleanup(_))));
        assert!(!f.settings.current().await.restart_recovery.enabled);
        let saved=f.data.read_settings_record().unwrap().unwrap();
        assert!(!saved.restart_recovery.as_ref().unwrap().effective().enabled);
        assert!(f.settings.policy().needs_cleanup());
        assert!(f.settings.engine.store.next_restart().await.unwrap().is_some());
        assert!(matches!(f.settings.change(recovery_change(true)).await,Err(SettingsError::Cleanup(_))));
        assert_eq!(f.data.read_settings_record().unwrap().unwrap(),saved,"On must not even save ahead of the failed Off cleanup");
        database.execute_batch("DROP TRIGGER fail_recovery_cleanup").unwrap();
        assert!(f.settings.change(recovery_change(true)).await.unwrap().restart_recovery.enabled);
        assert!(f.settings.engine.store.next_restart().await.unwrap().is_none());
        f.finish().await;
    }).await.unwrap();
}
#[tokio::test]
async fn failed_on_apply_pauses_admission_and_same_value_retry_retries_the_missing_commit() {
    tokio::time::timeout(WAIT, async {
        let f = Fixture::new(None).await;
        let database = rusqlite::Connection::open(f.data.database_path()).unwrap();
        database.execute_batch("CREATE TRIGGER fail_recovery_on BEFORE UPDATE ON restart_control WHEN NEW.enabled=1 BEGIN SELECT RAISE(ABORT,'test On apply failure'); END").unwrap();
        assert!(matches!(f.settings.change(recovery_change(true)).await,Err(SettingsError::Apply(_))));
        assert!(f.data.read_settings_record().unwrap().unwrap().restart_recovery.unwrap().effective().enabled);
        let (_,generation) = f.settings.policy().snapshot();
        assert!(!f.settings.policy().check(generation).unwrap());
        assert!(matches!(f.settings.change(recovery_change(true)).await,Err(SettingsError::Apply(_))),"an equal saved value cannot fabricate a successful runtime apply");
        database.execute_batch("DROP TRIGGER fail_recovery_on").unwrap();
        assert!(f.settings.change(SavedNodeSettings::default()).await.unwrap().restart_recovery.enabled);
        assert!(f.settings.policy().check(generation).unwrap());
        f.finish().await;
    }).await.unwrap();
}

#[tokio::test]
async fn failed_off_file_replacement_restores_admission_and_keeps_the_pending_intent() {
    tokio::time::timeout(WAIT, async {
        let f = Fixture::new(None).await;
        assert!(
            f.settings
                .change(recovery_change(true))
                .await
                .unwrap()
                .restart_recovery
                .enabled
        );
        let id = pending(&f).await;
        let saved = f.data.read_settings_record().unwrap().unwrap();
        let (_, generation) = f.settings.policy().snapshot();
        let path = f.data.settings_path();
        let backup = path.with_extension("saved-test");
        std::fs::rename(&path, &backup).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            f.settings.change(recovery_change(false)).await,
            Err(SettingsError::Write(_))
        ));
        assert!(f.settings.current().await.restart_recovery.enabled);
        assert!(f.settings.policy().check(generation).unwrap());
        assert_eq!(
            f.settings
                .engine
                .store
                .next_restart()
                .await
                .unwrap()
                .unwrap()
                .session,
            id
        );
        std::fs::remove_dir(&path).unwrap();
        std::fs::rename(&backup, &path).unwrap();
        assert_eq!(f.data.read_settings_record().unwrap().unwrap(), saved);
        f.finish().await;
    })
    .await
    .unwrap();
}
