//! Revocation is never acknowledged without reaching the consent file
//! (review row 318 F1): a restart must not revive a grant the person removed.
use super::*;
use std::os::unix::fs::PermissionsExt as _;

fn mode(path: &std::path::Path, bits: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap();
}

fn copilot(snapshot: &Snapshot) -> UsageSourceAvailability {
    snapshot
        .sources
        .as_slice()
        .iter()
        .find(|source| source.id.as_str() == "copilot")
        .unwrap()
        .availability
}

#[tokio::test]
async fn an_off_retry_after_a_failed_save_persists_off_across_a_restart() {
    vessel_testkit::within(
        TEST_BUDGET,
        Box::pin(async {
            let fixture = Fixture::start(|_name| None).await;
            let _on = fixture.allow(ExternalLookup::On).await;
            assert_eq!(
                fixture.dir().read_usage_sources().unwrap().as_slice().len(),
                1
            );
            // A read-only data directory refuses the atomic replacement's
            // temporary file, so this save fails with the old grant on disk.
            mode(fixture.data_root(), 0o500);
            let failed = fixture
                .sources
                .set(Fixture::request(ExternalLookup::Off), &fixture.global)
                .phase("usage phase: Off with a failing save")
                .await;
            mode(fixture.data_root(), 0o700);
            let failed = failed.expect_err(
                "precondition: a read-only data directory refuses the save (not run as root)",
            );
            assert_eq!(failed.code, ErrorCode::Internal);
            assert_eq!(
                fixture.dir().read_usage_sources().unwrap().as_slice().len(),
                1,
                "the failed save left the old grant in the file"
            );
            // Revocation already applies in memory.
            assert_eq!(
                copilot(&fixture.snapshot().await.unwrap()),
                UsageSourceAvailability::SourceOff
            );
            // The same Off again must reach the file before it is acknowledged.
            let retried = fixture.allow(ExternalLookup::Off).await;
            assert_eq!(retried.availability, UsageSourceAvailability::SourceOff);
            assert!(fixture.dir().read_usage_sources().unwrap().is_empty());
            fixture.finish().await;
            assert_eq!(
                copilot(&fixture.restarted_snapshot().await),
                UsageSourceAvailability::SourceOff,
                "a restart must not revive the revoked grant"
            );
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn an_unchanged_off_still_rewrites_a_file_that_was_never_saved_off() {
    vessel_testkit::within(
        TEST_BUDGET,
        Box::pin(async {
            let fixture = Fixture::start(|_name| None).await;
            let _on = fixture.allow(ExternalLookup::On).await;
            mode(fixture.data_root(), 0o500);
            let first = fixture
                .sources
                .set(Fixture::request(ExternalLookup::Off), &fixture.global)
                .phase("usage phase: first failing Off")
                .await;
            let second = fixture
                .sources
                .set(Fixture::request(ExternalLookup::Off), &fixture.global)
                .phase("usage phase: second failing Off")
                .await;
            mode(fixture.data_root(), 0o700);
            // Neither attempt may acknowledge while the file still grants it.
            assert_eq!(
                first.expect_err("first save must fail").code,
                ErrorCode::Internal
            );
            assert_eq!(
                second.expect_err("retry must not ack unsaved").code,
                ErrorCode::Internal
            );
            assert_eq!(
                fixture.dir().read_usage_sources().unwrap().as_slice().len(),
                1
            );
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}
