//! The mutation outlives its caller during the real write and after its save.
use super::*;

#[tokio::test]
async fn dropping_the_reply_during_real_write_still_applies_the_saved_value() {
    within(async {
        let f = Fixture::new(None).await;
        let (entered, release) = f.settings.engine.gates.arm_write();
        let settings = Arc::clone(&f.settings);
        let job = tokio::spawn(async move { settings.change(patch(idle())).await });
        step("writer entered", entered).await.unwrap();
        job.abort();
        assert!(job.await.unwrap_err().is_cancelled());
        release.send(()).unwrap();
        let current = step("owned write and apply", f.settings.current()).await;
        assert_eq!(current.park_after, idle());
        assert_eq!(f.runtime.park_after(), idle().duration());
        assert_eq!(
            f.data.read_settings_record().unwrap().unwrap().park_after,
            Some(idle())
        );
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn dropping_the_reply_after_save_cannot_abandon_runtime_apply() {
    within(async {
        let f = Fixture::new(None).await;
        let (entered, release) = f.settings.engine.gates.arm_apply();
        let settings = Arc::clone(&f.settings);
        let job = tokio::spawn(async move { settings.change(patch(idle())).await });
        step("saved before apply", entered).await.unwrap();
        assert_eq!(
            f.data.read_settings_record().unwrap().unwrap().park_after,
            Some(idle())
        );
        job.abort();
        assert!(job.await.unwrap_err().is_cancelled());
        release.send(()).unwrap();
        assert_eq!(
            step("apply without reply", f.settings.current())
                .await
                .park_after,
            idle()
        );
        assert_eq!(f.runtime.park_after(), idle().duration());
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn a_legacy_change_preserves_the_typed_saved_policy_and_prompt() {
    within(async {
        let saved = SavedNodeSettings {
            restart_recovery: Some(vessel_proto::restart::SetRestartRecovery {
                enabled: Some(true),
                prompt: Some("  synthetic continuation.\n".parse().unwrap()),
            }),
            ..SavedNodeSettings::default()
        };
        let f = Fixture::new(Some(saved.clone())).await;
        let answer = f.settings.change(patch(idle())).await.unwrap();
        assert!(answer.restart_recovery.enabled);
        assert_eq!(
            answer.restart_recovery.prompt.as_str(),
            "  synthetic continuation.\n"
        );
        assert_eq!(
            f.data
                .read_settings_record()
                .unwrap()
                .unwrap()
                .restart_recovery,
            saved.restart_recovery
        );
        f.finish().await;
    })
    .await;
}
