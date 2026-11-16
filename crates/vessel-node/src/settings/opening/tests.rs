//! The post-Store settings owner consumes the actual earlier snapshot unchanged.

use crate::settings::writer_tests::phase;
use std::sync::atomic::Ordering;

use super::*;
use crate::settings::Settings;
use crate::settings::writer_tests::{WRITE_TEST_BUDGET, runtime};

#[tokio::test]
async fn owner_consumes_the_pre_store_document_without_a_second_file_read() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        std::fs::write(
            data.settings_path(),
            br#"{"park_after":{"kind":"never"},"future":true}"#,
        )
        .unwrap();
        let opening = Opening::read_parts(data.clone(), ParkAfter::DEFAULT, false);
        assert_eq!(
            opening.disposition,
            SettingsDisposition::PreserveBeforeWrite
        );
        // Even a newly unreadable replacement cannot alter the already admitted
        // startup policy or cause a second set-aside decision in from_opening.
        let replacement = b"{ changed after the startup read";
        std::fs::write(data.settings_path(), replacement).unwrap();
        let settings = phase(
            "consume admitted boot snapshot",
            Settings::fixture_from_opening(opening, runtime(root.path())),
        )
        .await;
        assert_eq!(
            phase("read published settings", settings.current())
                .await
                .park_after,
            ParkAfter::Never
        );
        assert_eq!(settings.state.runtime.park_after(), None);
        assert!(settings.state.later.load(Ordering::Relaxed));
        assert!(settings.state.kept.is_none());
        assert_eq!(std::fs::read(data.settings_path()).unwrap(), replacement);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        phase("settings physical drain", settings.shutdown()).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn an_unreadable_startup_snapshot_is_moved_once_and_its_disposition_transfers() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let original = b"{ original unreadable settings";
        std::fs::write(data.settings_path(), original).unwrap();
        let opening = Opening::read_parts(data.clone(), ParkAfter::Never, false);
        assert_eq!(opening.disposition, SettingsDisposition::Current);
        assert!(opening.kept.is_none());
        let files = std::fs::read_dir(root.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        assert_eq!(std::fs::read(files.first().unwrap()).unwrap(), original);
        assert!(!data.settings_path().exists());
        let settings = phase(
            "consume admitted boot snapshot",
            Settings::fixture_from_opening(opening, runtime(root.path())),
        )
        .await;
        assert_eq!(
            phase("read published settings", settings.current())
                .await
                .park_after,
            ParkAfter::Never
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), files.len());
        phase("settings physical drain", settings.shutdown()).await;
    })
    .await
    .unwrap();
}
