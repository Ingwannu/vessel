use super::*;

#[test]
fn unknown_or_missing_live_token_is_not_process_stop_proof() {
    let token = CancellationToken::new();
    assert_eq!(
        cancel_observed(SetupRunState::Unknown, Some(&token)),
        CancelSetupOutcome::Unknown
    );
    assert!(!token.is_cancelled());
    assert_eq!(
        cancel_observed(SetupRunState::Running, None),
        CancelSetupOutcome::Unknown
    );
    for state in [
        SetupRunState::Succeeded,
        SetupRunState::Failed,
        SetupRunState::Cancelled,
    ] {
        assert_eq!(
            cancel_observed(state, Some(&token)),
            CancelSetupOutcome::NotRunning
        );
        assert!(!token.is_cancelled());
    }
    assert_eq!(
        cancel_observed(SetupRunState::Running, Some(&token)),
        CancelSetupOutcome::Requested
    );
    assert!(token.is_cancelled());
}

#[tokio::test]
async fn structured_events_do_not_wake_legacy_log_readers_without_byte_or_terminal_progress() {
    use crate::workspace::setup::Observation;
    use std::sync::Arc;
    use vessel_proto::workspace::{SetupState, WorktreeInfo};
    use vessel_proto::{Creator, ProjectId};
    use vessel_workspace::commands::{self, Run};
    let folder = tempfile::tempdir().unwrap();
    let id = WorktreeId::random();
    let commands = ["printf ready".parse().unwrap()];
    let mut entry = WorktreeEntry::new(WorktreeInfo {
        id,
        project: ProjectId::random(),
        path: folder.path().to_path_buf(),
        branch: None,
        created_branch: None,
        base: None,
        created_at: chrono::DateTime::UNIX_EPOCH,
        created_by: Creator::Owner,
        created_for: None,
        sessions: vec![],
        setup: SetupState::None,
        kept: None,
    });
    let observation = Observation::new(id, &commands).unwrap();
    let generation = observation.snapshot.cursor.run;
    entry.observation = Some(observation);
    let written = Arc::clone(&entry.written);
    let mut legacy = entry.written.subscribe();
    let mut structured = entry.observed.subscribe();
    let log = folder.path().join("observer-isolation.log");
    let run = Run {
        dir: folder.path(),
        env: vec![],
        removed: &[],
        log: &log,
        scratch: folder.path(),
        limit: commands::SETUP_LIMIT,
        cancel: CancellationToken::new(),
    };
    let mut saw_start = false;
    assert_eq!(
        commands::run_observed(&commands, &run, &written, |event| {
            drop(legacy.borrow_and_update());
            drop(structured.borrow_and_update());
            observe_entry(&mut entry, SetupRunId::random(), event);
            assert!(
                !structured.has_changed().unwrap(),
                "stale invocation cannot publish"
            );
            observe_entry(&mut entry, generation, event);
            assert!(
                !legacy.has_changed().unwrap(),
                "structured notification is not legacy byte/state progress"
            );
            assert!(structured.has_changed().unwrap());
            if matches!(event, CommandEvent::Started { .. }) {
                saw_start = true;
                assert_eq!(
                    *legacy.borrow(),
                    0,
                    "start precedes the first legacy log line"
                );
            }
        })
        .await
        .unwrap(),
        None
    );
    assert!(saw_start);
    assert!(
        *legacy.borrow() > 0,
        "ordinary log writes still notify their reader"
    );
}
