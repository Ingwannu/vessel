//! One process owns this callsite's subscriber from its first registration.
//! Running beside unrelated LIB tests can cache disabled interest before this
//! thread-local subscriber exists; process isolation preserves the assertion.
use std::time::Duration;
use vessel_proc::{CommandDiagnostics, CommandSpec, spawn};
use vessel_testkit::Fake;

#[cfg(test)]
#[path = "private_spawn_log/capture.rs"]
mod capture;
const SENTINEL: &str = "PRIVATE-CREATION-PROMPT-CANARY";
const DEADLINE: Duration = Duration::from_secs(30);

#[tokio::test]
async fn actual_process_spawn_log_uses_the_private_label_and_still_executes() {
    vessel_testkit::within(DEADLINE, async {
        let program = Fake::Tui.path().unwrap();
        let spec = CommandSpec::new(&program)
            .args(["--version", SENTINEL])
            .with_diagnostics(CommandDiagnostics::Redacted);
        let capture = capture::Capture::default();
        let mut spawned =
            tracing::subscriber::with_default(capture.clone(), || spawn(&spec)).unwrap();
        assert_eq!(spawned.child.wait().await.unwrap(), Some(0));
        let events = capture.text();
        assert!(events.contains("spawned"), "actual spawn event missing");
        assert!(events.contains("<redacted>"));
        assert!(!events.contains(SENTINEL));
        assert!(!events.contains(&program.display().to_string()));
    })
    .await
    .unwrap();
}
