//! Typed fake receipts exercise presentation policy, not a node or process tree.
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use vessel_proto::workspace::{
    ProjectCommand, SetupCommandProgress, SetupCommandResult, SetupCursor,
};
use vessel_proto::{DurationMs, NodeId, SetupRevision, TerminalBytes};

fn address() -> Address {
    Address {
        context: ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), NodeId::random())
            .unwrap(),
        worktree: WorktreeId::random(),
    }
}
fn progress(address: &Address) -> SetupProgress {
    SetupProgress {
        worktree: address.worktree,
        cursor: SetupCursor {
            run: SetupRunId::random(),
            revision: SetupRevision::ZERO,
        },
        state: SetupRunState::Running,
        started_at: chrono::DateTime::UNIX_EPOCH,
        commands: ["first", "last"]
            .into_iter()
            .enumerate()
            .map(|(position, command)| SetupCommandProgress {
                index: SetupCommandIndex::try_from(
                    u32::try_from(position.checked_add(1).unwrap()).unwrap(),
                )
                .unwrap(),
                command: command.parse::<ProjectCommand>().unwrap(),
                state: SetupCommandState::Waiting,
                output: TerminalBytes::default(),
                truncated: false,
            })
            .collect(),
    }
}
fn apply(model: &mut Model, address: &Address, snapshot: SetupProgress) {
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(
        address,
        key,
        Ok(SetupProgressReply {
            progress: Some(snapshot)
        })
    ));
}
#[test]
fn unavailable_is_default_and_absence_does_not_become_success_or_cancel_authority() {
    let mut model = Model::default();
    let address = address();
    model.select(Some(address.clone()), Support::Unavailable);
    assert!(model.begin_read().is_none());
    model.select(Some(address.clone()), Support::ObserveAndCancel);
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&address, key, Ok(SetupProgressReply { progress: None })));
    assert!(model.snapshot().is_none());
    assert!(!model.can_cancel());
}
#[test]
fn running_then_last_auto_expansion_preserves_manual_collapse_until_new_run() {
    let address = address();
    let mut snapshot = progress(&address);
    snapshot.commands[0].state = SetupCommandState::Running;
    let first = snapshot.commands[0].index;
    let last = snapshot.commands[1].index;
    let mut model = Model::default();
    model.select(Some(address.clone()), Support::Observe);
    apply(&mut model, &address, snapshot.clone());
    assert!(model.expanded(first));
    model.toggle(snapshot.cursor.run, first);
    assert!(!model.expanded(first));
    apply(&mut model, &address, snapshot.clone());
    assert!(!model.expanded(first));
    snapshot.commands[0].state = SetupCommandState::Finished {
        elapsed: DurationMs::from_duration(std::time::Duration::from_millis(1200)),
        result: SetupCommandResult::Exited { code: Some(0) },
    };
    apply(&mut model, &address, snapshot.clone());
    assert_eq!(model.automatic(), Some(first));
    assert!(!model.expanded(first));
    assert!(!model.expanded(last));
    snapshot.commands[1].state = SetupCommandState::Running;
    apply(&mut model, &address, snapshot.clone());
    assert!(model.expanded(last));
    snapshot.commands[1].state = SetupCommandState::Waiting;
    snapshot.cursor.run = SetupRunId::random();
    snapshot.commands[0].state = SetupCommandState::Running;
    apply(&mut model, &address, snapshot);
    assert!(model.expanded(first));
}
#[test]
fn requested_is_not_a_terminal_state_and_stale_cancel_cannot_settle_a_new_run() {
    let address = address();
    let first = progress(&address);
    let mut model = Model::default();
    model.select(Some(address.clone()), Support::ObserveAndCancel);
    apply(&mut model, &address, first.clone());
    let (key, cancel) = model.begin_cancel(first.cursor.run).unwrap();
    assert_eq!(cancel.run, first.cursor.run);
    assert!(model.accept_cancel(&address, key, cancel.run, Ok(CancelSetupOutcome::Requested)));
    assert_eq!(model.snapshot().unwrap().state, SetupRunState::Running);
    assert!(!model.can_cancel());
    let next = progress(&address);
    apply(&mut model, &address, next.clone());
    assert!(!model.accept_cancel(&address, key, cancel.run, Ok(CancelSetupOutcome::Requested)));
    assert_eq!(model.snapshot().unwrap().cursor, next.cursor);
    assert!(model.can_cancel());
    assert!(model.begin_cancel(first.cursor.run).is_none());
    assert!(model.begin_cancel(next.cursor.run).is_some());
}
#[test]
fn final_cr_segment_matches_the_reference_and_does_not_preserve_old_progress_text() {
    assert_eq!(
        output(b"Downloading\rOk\n10%\r100% done\n"),
        "Ok\n100% done\n"
    );
    assert_eq!(output("시작\r완료".as_bytes()), "완료");
}

#[test]
fn elapsed_duration_uses_the_reference_whole_unit_boundaries() {
    for (ms, text) in [
        (0, "0ms"),
        (999, "999ms"),
        (1000, "1s"),
        (59999, "59s"),
        (60000, "1m 0s"),
        (61000, "1m 1s"),
    ] {
        assert_eq!(
            elapsed(DurationMs::from_duration(std::time::Duration::from_millis(
                ms
            ))),
            text
        );
    }
}

#[test]
fn waiting_rows_never_displace_the_failed_command_output() {
    let address = address();
    let mut snapshot = progress(&address);
    let mut model = Model::default();
    model.select(Some(address.clone()), Support::Observe);
    apply(&mut model, &address, snapshot.clone());
    assert_eq!(model.automatic(), None);
    snapshot.state = SetupRunState::Failed;
    snapshot.commands[0].state = SetupCommandState::Finished {
        elapsed: DurationMs::from_duration(std::time::Duration::from_millis(1)),
        result: SetupCommandResult::Exited { code: Some(7) },
    };
    let attempted = snapshot.commands[0].index;
    let waiting = snapshot.commands[1].index;
    apply(&mut model, &address, snapshot);
    assert_eq!(model.automatic(), Some(attempted));
    assert!(model.expanded(attempted));
    assert!(!model.expanded(waiting));
    assert!(!model.can_cancel());
}
