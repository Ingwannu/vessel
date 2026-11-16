//! Capacity and ownership are tested while the real blocking writer is held.

use crate::settings::writer_tests::phase;
use std::task::Poll;

use vessel_proto::parking::ParkAfter;

use super::*;
use crate::settings::writer_tests::{WRITE_TEST_BUDGET, patch, settings};

#[tokio::test]
async fn waiting_capacity_refuses_overflow_and_dropped_replies_still_apply_in_order() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = phase("open real settings owner", settings(root.path())).await;
        let (entered, release) = settings.state.gate.install();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(patch(ParkAfter::Never)).await });
        phase("enter real blocking writer", entered).await.unwrap();
        caller.abort();
        assert!(
            phase("cancelled caller joins", caller)
                .await
                .unwrap_err()
                .is_cancelled()
        );
        let mut replies = Vec::new();
        for index in 0..SettingsAdmission::MAX_WAITING {
            let next = if index.checked_add(1) == Some(SettingsAdmission::MAX_WAITING) {
                ParkAfter::DEFAULT
            } else {
                ParkAfter::Never
            };
            replies.push(settings.admission.submit(patch(next)).unwrap());
        }
        let error = settings
            .admission
            .submit(patch(ParkAfter::Never))
            .unwrap_err();
        assert!(matches!(error, SettingsError::Full));
        assert!(
            error
                .to_string()
                .contains(&SettingsAdmission::MAX_WAITING.to_string())
        );
        assert!(settings.state.saved.try_lock().is_err());
        assert!(!settings.state.data_dir.settings_path().exists());
        // In particular, losing the final queued reply must not lose its patch.
        drop(replies);
        let mut stopping = Box::pin(settings.shutdown());
        assert!(matches!(futures_util::poll!(&mut stopping), Poll::Pending));
        assert!(matches!(
            phase(
                "apply parking patch",
                settings.change(patch(ParkAfter::Never))
            )
            .await,
            Err(SettingsError::Stopping)
        ));
        drop(stopping);
        release.send(()).unwrap();
        phase("settings physical drain", settings.shutdown()).await;
        assert_eq!(
            phase("read published settings", settings.current())
                .await
                .park_after,
            ParkAfter::DEFAULT
        );
        assert_eq!(
            settings.state.runtime.park_after(),
            ParkAfter::DEFAULT.duration()
        );
        assert_eq!(
            settings
                .state
                .data_dir
                .read_settings()
                .unwrap()
                .unwrap()
                .public_settings()
                .park_after,
            Some(ParkAfter::DEFAULT)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn the_consumer_outlives_the_last_handle_but_does_not_retain_itself() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = phase("open real settings owner", settings(root.path())).await;
        let state = Arc::downgrade(&settings.state);
        let tasks = settings.admission.tasks.clone();
        let data = settings.state.data_dir.clone();
        let (entered, release) = settings.state.gate.install();
        let reply = settings.admission.submit(patch(ParkAfter::Never)).unwrap();
        phase("enter real blocking writer", entered).await.unwrap();
        let last = settings
            .admission
            .submit(patch(ParkAfter::DEFAULT))
            .unwrap();
        drop(reply);
        drop(last);
        drop(settings);
        let _first_close = tasks.close();
        let mut drained = Box::pin(tasks.wait());
        assert!(matches!(futures_util::poll!(&mut drained), Poll::Pending));
        assert!(state.upgrade().is_some());
        release.send(()).unwrap();
        phase("last handle physical drain", drained).await;
        assert!(state.upgrade().is_none());
        assert_eq!(
            data.read_settings()
                .unwrap()
                .unwrap()
                .public_settings()
                .park_after,
            Some(ParkAfter::DEFAULT)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn queued_patches_are_prepared_from_the_preceding_publication() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = phase("open real settings owner", settings(root.path())).await;
        let (entered, release) = settings.state.gate.install();
        let first = settings.admission.submit(patch(ParkAfter::Never)).unwrap();
        phase("enter real blocking writer", entered).await.unwrap();
        // An empty patch must observe the newly saved value, not a snapshot from
        // when this intent was accepted behind the held replacement.
        let next = settings
            .admission
            .submit(SetNodeSettings::default())
            .unwrap();
        release.send(()).unwrap();
        let applied = phase("first accepted reply", first).await.unwrap().unwrap();
        assert_eq!(
            phase("following accepted reply", next)
                .await
                .unwrap()
                .unwrap(),
            applied
        );
        phase("settings physical drain", settings.shutdown()).await;
        assert_eq!(applied.park_after, ParkAfter::Never);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn task_panics_keep_the_concrete_cause_private_at_every_public_error_edge() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        const MARKER: &str = "private-settings-task-marker-6fef";
        let cause = phase(
            "panic task joins with sealed cause",
            tokio::spawn(async { panic!("{MARKER}") }),
        )
        .await
        .unwrap_err();
        let error = SettingsError::Lost(super::super::writer::TaskFailure::from(cause));
        let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
        while let Some(error) = source {
            assert!(!error.to_string().contains(MARKER));
            assert!(!format!("{error:?}").contains(MARKER));
            source = error.source();
        }
        assert!(error.to_string().contains("settings task panicked"));
    })
    .await
    .unwrap();
}
