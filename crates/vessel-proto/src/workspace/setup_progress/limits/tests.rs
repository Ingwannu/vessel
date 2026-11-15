use super::*;
use crate::workspace::{SetupCommandIndex, SetupCommandState, SetupCursor, SetupRunState};
use crate::{SetupRevision, SetupRunId, TerminalBytes, WorktreeId};

fn snapshot() -> SetupProgress {
    SetupProgress {
        worktree: WorktreeId::random(),
        cursor: SetupCursor {
            run: SetupRunId::random(),
            revision: SetupRevision::ZERO,
        },
        state: SetupRunState::Running,
        started_at: chrono::Utc::now(),
        commands: (0..MAX_PROJECT_COMMANDS)
            .map(|position| SetupCommandProgress {
                index: SetupCommandIndex::try_from(
                    u32::try_from(position.checked_add(1).unwrap()).unwrap(),
                )
                .unwrap(),
                command: "echo setup".parse().unwrap(),
                state: SetupCommandState::Waiting,
                output: TerminalBytes::default(),
                truncated: false,
            })
            .collect(),
    }
}

fn decoding_refuses(progress: &SetupProgress) {
    let wire = serde_json::to_string(progress).unwrap();
    assert!(serde_json::from_str::<SetupProgress>(&wire).is_err());
}

#[test]
fn full_command_and_output_budgets_roundtrip_at_the_exact_boundary() {
    let mut progress = snapshot();
    for command in &mut progress.commands {
        command.output = TerminalBytes::new(vec![b'x'; MAX_SETUP_COMMAND_OUTPUT_BYTES].into());
    }
    assert_eq!(progress.validate(), Ok(()));
    assert_eq!(
        progress
            .commands
            .iter()
            .map(|command| command.output.bytes().len())
            .sum::<usize>(),
        MAX_SETUP_OUTPUT_BYTES,
    );
    let wire = serde_json::to_string(&progress).unwrap();
    assert_eq!(
        serde_json::from_str::<SetupProgress>(&wire).unwrap(),
        progress
    );
}

#[test]
fn one_extra_output_byte_is_rejected_by_local_and_wire_admission() {
    let mut progress = snapshot();
    let extra = MAX_SETUP_COMMAND_OUTPUT_BYTES.checked_add(1).unwrap();
    progress.commands.first_mut().unwrap().output = TerminalBytes::new(vec![b'x'; extra].into());
    assert_eq!(progress.validate(), Err(InvalidSetupProgress::Output));
    decoding_refuses(&progress);
}

#[test]
fn count_duplicate_and_reordered_rows_cannot_be_admitted_as_a_snapshot() {
    let mut progress = snapshot();
    progress
        .commands
        .push(progress.commands.first().unwrap().clone());
    assert_eq!(progress.validate(), Err(InvalidSetupProgress::Commands));
    decoding_refuses(&progress);

    let mut duplicate = snapshot();
    duplicate.commands.get_mut(1).unwrap().index = duplicate.commands.first().unwrap().index;
    assert_eq!(duplicate.validate(), Err(InvalidSetupProgress::Order));
    decoding_refuses(&duplicate);

    let mut reordered = snapshot();
    reordered.commands.swap(0, 1);
    assert_eq!(reordered.validate(), Err(InvalidSetupProgress::Order));
    decoding_refuses(&reordered);
}
