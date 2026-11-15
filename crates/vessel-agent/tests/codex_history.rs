//! Actual fake-process listing uses no thread/turn or provider filesystem intake.
use std::time::Duration;
use vessel_agent::{
    CommandSpec, Driver, HistoryError, HistoryFrom, HistoryTimestamp, history_with,
};
use vessel_testkit::Fake;
const PROBE_DEADLINE: Duration = Duration::from_secs(5);

#[tokio::test]
async fn native_history_process_reads_pages_without_creating_or_resuming_a_thread() {
    let home = tempfile::tempdir().unwrap();
    let audit = home.path().join("methods");
    let command = CommandSpec::new(Fake::CodexAppServer.path().unwrap())
        .arg("app-server")
        .env_clear()
        .env("HOME", home.path())
        .env("CODEX_HOME", home.path())
        .env("FAKE_CODEX_HISTORY_AUDIT", &audit);
    let page = history_with(
        Driver::CodexAppServer,
        &command,
        Some(std::path::Path::new("/work")),
        HistoryFrom::default(),
        PROBE_DEADLINE,
    )
    .await
    .unwrap();
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.entries[0].session.as_str(), "native-actual");
    assert_eq!(
        page.entries[0].updated_at,
        Some(HistoryTimestamp::UnixSeconds(123))
    );
    assert_eq!(page.entries[1].updated_at, None);
    assert_eq!(
        std::fs::read_to_string(audit).unwrap(),
        "initialize\ninitialized\nthread/list\nthread/list\n"
    );
}
#[tokio::test]
async fn explicit_missing_method_and_malformed_response_have_distinct_outcomes() {
    let home = tempfile::tempdir().unwrap();
    for mode in ["unsupported", "invalid"] {
        let command = CommandSpec::new(Fake::CodexAppServer.path().unwrap())
            .arg("app-server")
            .env_clear()
            .env("HOME", home.path())
            .env("CODEX_HOME", home.path())
            .env("FAKE_CODEX_HISTORY", mode);
        let result = history_with(
            Driver::CodexAppServer,
            &command,
            None,
            HistoryFrom::default(),
            PROBE_DEADLINE,
        )
        .await;
        if mode == "unsupported" {
            assert!(matches!(result, Err(HistoryError::CannotList)));
        } else {
            assert!(matches!(result, Err(HistoryError::Failed { .. })));
        }
    }
}
