//! Real runner + registry behavior, isolated from user git/home and agents.

use super::*;
use vessel_proto::SetupRunId;
use vessel_proto::workspace::{
    CancelSetup, CancelSetupOutcome, ReadSetupProgress, SetupCommandResult, SetupCommandState,
    SetupProgress, SetupRunState,
};

const OBSERVATION_ROUNDS: usize = 40;

async fn snapshot(workspaces: &Workspaces, worktree: vessel_proto::WorktreeId) -> SetupProgress {
    workspaces
        .setup_progress(ReadSetupProgress {
            worktree,
            after: None,
            wait_ms: None,
        })
        .await
        .unwrap()
        .progress
        .unwrap()
}

async fn until(
    workspaces: &Workspaces,
    worktree: vessel_proto::WorktreeId,
    ready: impl Fn(&SetupProgress) -> bool,
) -> SetupProgress {
    let mut progress = snapshot(workspaces, worktree).await;
    for _round in 0..OBSERVATION_ROUNDS {
        if ready(&progress) {
            return progress;
        }
        progress = workspaces
            .setup_progress(ReadSetupProgress {
                worktree,
                after: Some(progress.cursor),
                wait_ms: Some(vessel_proto::DurationMs::from_duration(
                    std::time::Duration::from_secs(1),
                )),
            })
            .await
            .unwrap()
            .progress
            .unwrap();
    }
    panic!("setup observation never reached expected state: {progress:?}");
}

async fn approved_worktree(fixture: &Fixture) -> WorktreeInfo {
    let project = fixture
        .project(
            "setup-observation",
            Some("[worktree]\nsetup = [\"printf ready; sleep 30\", \"touch never\"]\n"),
        )
        .await;
    let asked = fixture
        .create(project.id, Creator::Owner, SetupChoice::Run)
        .await;
    let CreateWorktreeOutcome::NeedsApproval { commands } = asked else {
        panic!("{asked:?}")
    };
    let ProjectCommands::Listed { digest, .. } = commands else {
        panic!("{commands:?}")
    };
    let _approved = fixture
        .workspaces
        .approve(project.id, digest, true)
        .await
        .unwrap();
    fixture.created(project.id).await
}

fn assert_approved_ordinals(progress: &SetupProgress) {
    for (position, command) in progress.commands.iter().enumerate() {
        assert_eq!(
            command.index.get(),
            u32::try_from(position.checked_add(1).unwrap()).unwrap()
        );
    }
}

#[tokio::test]
async fn cancellation_is_generation_specific_and_restart_does_not_invent_command_history() {
    let fixture = Fixture::new().await;
    let worktree = approved_worktree(&fixture).await;
    let running = until(&fixture.workspaces, worktree.id, |progress| {
        progress
            .commands
            .first()
            .is_some_and(|command| !command.output.bytes().is_empty())
    })
    .await;
    assert_eq!(running.state, SetupRunState::Running);
    assert_eq!(running.commands.len(), 2);
    assert_approved_ordinals(&running);
    assert_eq!(
        fixture
            .workspaces
            .cancel_setup(CancelSetup {
                worktree: worktree.id,
                run: SetupRunId::random(),
            })
            .unwrap(),
        CancelSetupOutcome::Stale
    );
    assert_eq!(
        snapshot(&fixture.workspaces, worktree.id).await.state,
        SetupRunState::Running
    );
    assert_eq!(
        fixture
            .workspaces
            .cancel_setup(CancelSetup {
                worktree: worktree.id,
                run: running.cursor.run,
            })
            .unwrap(),
        CancelSetupOutcome::Requested
    );
    let stopped = until(&fixture.workspaces, worktree.id, |progress| {
        progress.state != SetupRunState::Running
    })
    .await;
    assert_eq!(stopped.state, SetupRunState::Cancelled);
    let [first, second] = stopped.commands.as_slice() else {
        panic!("unexpected command rows");
    };
    assert!(matches!(
        first.state,
        SetupCommandState::Finished {
            result: SetupCommandResult::Cancelled,
            ..
        }
    ));
    assert_eq!(second.state, SetupCommandState::Waiting);
    assert!(!worktree.path.join("never").exists());
    assert_eq!(
        fixture
            .workspaces
            .cancel_setup(CancelSetup {
                worktree: worktree.id,
                run: running.cursor.run,
            })
            .unwrap(),
        CancelSetupOutcome::NotRunning
    );

    let _rerun = fixture.workspaces.run_setup(worktree.id).await.unwrap();
    let newer = snapshot(&fixture.workspaces, worktree.id).await;
    assert_ne!(newer.cursor.run, running.cursor.run);
    assert_eq!(
        fixture
            .workspaces
            .cancel_setup(CancelSetup {
                worktree: worktree.id,
                run: running.cursor.run,
            })
            .unwrap(),
        CancelSetupOutcome::Stale
    );
    let _requested = fixture
        .workspaces
        .cancel_setup(CancelSetup {
            worktree: worktree.id,
            run: newer.cursor.run,
        })
        .unwrap();
    let _stopped = until(&fixture.workspaces, worktree.id, |progress| {
        progress.state != SetupRunState::Running
    })
    .await;
    let restarted = fixture.restart().await;
    assert_eq!(
        restarted
            .setup_progress(ReadSetupProgress {
                worktree: worktree.id,
                after: None,
                wait_ms: None,
            })
            .await
            .unwrap()
            .progress,
        None
    );
}

#[tokio::test]
async fn structured_events_do_not_wake_legacy_log_readers_without_byte_or_terminal_progress() {
    use crate::workspace::setup::Observation;
    use vessel_workspace::commands::{self, CommandEvent, Run};
    let fixture = Fixture::new().await;
    let project = fixture.project("separate-observation-watch", None).await;
    let worktree = fixture.created(project.id).await;
    let commands = ["printf ready".parse().unwrap()];
    let observation = Observation::new(worktree.id, &commands).unwrap();
    let generation = observation.snapshot.cursor.run;
    let (written, mut legacy, mut structured) = {
        let mut state = fixture.workspaces.state();
        let entry = state.worktrees.get_mut(&worktree.id).unwrap();
        entry.observation = Some(observation);
        (
            Arc::clone(&entry.written),
            entry.written.subscribe(),
            entry.observed.subscribe(),
        )
    };
    let log = fixture.temp.path().join("observer-isolation.log");
    let run = Run {
        dir: &worktree.path,
        env: fixture.env.clone(),
        removed: &[],
        log: &log,
        scratch: fixture.temp.path(),
        limit: commands::SETUP_LIMIT,
        cancel: CancellationToken::new(),
    };
    let mut saw_start = false;
    assert_eq!(
        commands::run_observed(&commands, &run, &written, |event| {
            // A real runner can already have published log bytes. Mark those seen,
            // then verify that this structured event alone cannot wake that reader.
            drop(legacy.borrow_and_update());
            drop(structured.borrow_and_update());
            fixture
                .workspaces
                .observe_setup(worktree.id, generation, event);
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
