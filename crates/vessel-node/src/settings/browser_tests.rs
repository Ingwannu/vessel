//! The actual shared settings/file owner, with no browser engine or tool lease.
use super::*;
use std::task::Poll;
use std::time::Duration;
use vessel_proto::browser::BrowserTools;
const FIXTURE_BUDGET: Duration = Duration::from_secs(20);
fn patch(choice: BrowserTools) -> SetNodeSettings {
    SetNodeSettings {
        browser_tools: Some(choice),
        ..SetNodeSettings::default()
    }
}
#[tokio::test]
async fn missing_off_and_known_choices_survive_restart_without_a_browser_runtime() {
    tokio::time::timeout(FIXTURE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = writer_tests::settings(root.path());
        assert_eq!(settings.current().await.browser_tools, BrowserTools::Off);
        for choice in [BrowserTools::On, BrowserTools::Off] {
            assert_eq!(
                settings.change(patch(choice)).await.unwrap().browser_tools,
                choice
            );
            assert_eq!(
                settings
                    .data_dir
                    .read_settings()
                    .unwrap()
                    .unwrap()
                    .browser_tools,
                Some(choice)
            );
            let reopened = writer_tests::settings(root.path());
            assert_eq!(reopened.current().await.browser_tools, choice);
            reopened.shutdown().await;
        }
        settings.shutdown().await;
        // This test intentionally proves saved preference only. There is no
        // actual browser runtime, provider start or offered-tool producer here.
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn concurrent_unrelated_change_preserves_the_browser_choice() {
    tokio::time::timeout(FIXTURE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = writer_tests::settings(root.path());
        let parking = SetNodeSettings {
            park_after: Some(ParkAfter::Never),
            ..SetNodeSettings::default()
        };
        let (browser, park) = tokio::join!(
            settings.change(patch(BrowserTools::On)),
            settings.change(parking)
        );
        browser.unwrap();
        park.unwrap();
        let current = settings.current().await;
        assert_eq!(current.browser_tools, BrowserTools::On);
        assert_eq!(current.park_after, ParkAfter::Never);
        let saved = settings.data_dir.read_settings().unwrap().unwrap();
        assert_eq!(saved.browser_tools, Some(BrowserTools::On));
        assert_eq!(saved.park_after, Some(ParkAfter::Never));
        settings.shutdown().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn unknown_named_choice_cannot_partially_apply_an_unrelated_field_or_write() {
    tokio::time::timeout(FIXTURE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = writer_tests::settings(root.path());
        settings.change(patch(BrowserTools::On)).await.unwrap();
        let before = std::fs::read(settings.data_dir.settings_path()).unwrap();
        let invalid = SetNodeSettings {
            browser_tools: Some(BrowserTools::Unknown),
            park_after: Some(ParkAfter::Never),
            ..SetNodeSettings::default()
        };
        assert!(matches!(
            settings.change(invalid).await,
            Err(SettingsError::UnknownBrowser)
        ));
        assert_eq!(
            std::fs::read(settings.data_dir.settings_path()).unwrap(),
            before
        );
        assert_eq!(settings.current().await.browser_tools, BrowserTools::On);
        assert_eq!(settings.current().await.park_after, ParkAfter::DEFAULT);
        settings.shutdown().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn failed_real_write_does_not_publish_browser_on() {
    tokio::time::timeout(FIXTURE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = writer_tests::settings(root.path());
        // A directory cannot be atomically replaced by the settings file. This
        // fails for privileged test users too, without modifying permissions.
        std::fs::create_dir(settings.data_dir.settings_path()).unwrap();
        assert!(matches!(
            settings.change(patch(BrowserTools::On)).await,
            Err(SettingsError::Write(_))
        ));
        assert_eq!(settings.current().await.browser_tools, BrowserTools::Off);
        assert_eq!(settings.saved.lock().await.browser_tools, None);
        assert!(settings.data_dir.settings_path().is_dir());
        settings.shutdown().await;
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn unknown_saved_choice_is_off_and_original_later_file_is_preserved() {
    tokio::time::timeout(FIXTURE_BUDGET,async {
        let root=tempfile::tempdir().unwrap();
        let data=DataDir::new(root.path());
        let original=br#"{"browser_tools":"future-browser-mode","future_nested":{"marker":"future-choice-canary"}}"#;
        std::fs::write(data.settings_path(),original).unwrap();
        let settings=writer_tests::settings(root.path());
        assert_eq!(settings.current().await.browser_tools,BrowserTools::Off);
        settings.change(SetNodeSettings {park_after:Some(ParkAfter::Never),..SetNodeSettings::default()}).await.unwrap();
        assert_eq!(settings.current().await.browser_tools,BrowserTools::Off);
        let preserved=std::fs::read_dir(root.path()).unwrap().filter_map(Result::ok)
            .map(|entry|entry.path()).find(|path|path.file_name().unwrap().to_string_lossy().starts_with("settings.json.later-")).unwrap();
        assert_eq!(std::fs::read(preserved).unwrap(),original);
        assert_eq!(settings.data_dir.read_settings().unwrap().unwrap().browser_tools,Some(BrowserTools::Unknown));
        settings.shutdown().await;
    }).await.unwrap();
}
#[tokio::test]
async fn cancelled_browser_change_keeps_actual_writer_guard_and_unrelated_successor() {
    tokio::time::timeout(FIXTURE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = writer_tests::settings(root.path());
        let (entered, release) = settings.gate.install();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(patch(BrowserTools::On)).await });
        entered.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        assert!(settings.saved.try_lock().is_err());
        let mut next = Box::pin(settings.change(SetNodeSettings {
            park_after: Some(ParkAfter::Never),
            ..SetNodeSettings::default()
        }));
        assert!(matches!(futures_util::poll!(&mut next), Poll::Pending));
        release.send(()).unwrap();
        let applied = next.await.unwrap();
        settings.shutdown().await;
        assert_eq!(applied.browser_tools, BrowserTools::On);
        assert_eq!(applied.park_after, ParkAfter::Never);
        assert_eq!(settings.current().await, applied);
        assert_eq!(
            settings
                .data_dir
                .read_settings()
                .unwrap()
                .unwrap()
                .browser_tools,
            Some(BrowserTools::On)
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            let file = std::fs::metadata(settings.data_dir.settings_path()).unwrap();
            let parent = std::fs::metadata(root.path()).unwrap();
            assert_eq!(file.uid(), parent.uid());
            assert_eq!(file.gid(), parent.gid());
            assert_eq!(file.mode() & 0o077, 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn malformed_saved_object_cannot_enable_tools_and_original_bytes_are_preserved() {
    tokio::time::timeout(FIXTURE_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let original = br#"{"browser_tools":{"on":null}}"#;
        std::fs::write(data.settings_path(), original).unwrap();
        let settings = writer_tests::settings(root.path());
        assert_eq!(settings.current().await.browser_tools, BrowserTools::Off);
        assert!(!data.settings_path().exists());
        let preserved = std::fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("settings.json.unreadable-")
            })
            .unwrap();
        assert_eq!(std::fs::read(preserved).unwrap(), original);
        settings
            .change(SetNodeSettings {
                park_after: Some(ParkAfter::Never),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(settings.current().await.browser_tools, BrowserTools::Off);
        settings.shutdown().await;
    })
    .await
    .unwrap();
}
