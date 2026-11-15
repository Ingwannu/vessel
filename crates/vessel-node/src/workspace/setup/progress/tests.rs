use super::*;

fn row() -> SetupCommandProgress {
    SetupCommandProgress {
        index: SetupCommandIndex::try_from(0).unwrap(),
        command: "echo setup".parse().unwrap(),
        state: SetupCommandState::Running,
        output: TerminalBytes::default(),
        truncated: false,
    }
}

#[test]
fn middle_truncation_is_bounded_and_retains_the_latest_tail_across_chunks() {
    let mut row = row();
    append_output(&mut row, b"first\n");
    append_output(&mut row, &vec![b'x'; MAX_COMMAND_OUTPUT]);
    append_output(&mut row, b"\nlast\n");
    let bytes = row.output.bytes();
    assert!(row.truncated);
    assert_eq!(bytes.len(), MAX_COMMAND_OUTPUT);
    assert!(bytes.starts_with(b"first\n"));
    assert!(bytes.ends_with(b"\nlast\n"));
    assert!(
        bytes
            .windows(MIDDLE_MARKER.len())
            .any(|part| part == MIDDLE_MARKER)
    );
    append_output(&mut row, &vec![b'z'; MAX_COMMAND_OUTPUT.saturating_mul(4)]);
    assert_eq!(row.output.bytes().len(), MAX_COMMAND_OUTPUT);
    assert!(row.output.bytes().starts_with(b"first\n"));
    assert!(row.output.bytes().ends_with(b"zzzz"));
}

#[test]
fn exactly_the_limit_is_complete_and_unknown_rows_never_become_success() {
    let mut row = row();
    append_output(&mut row, &vec![b'x'; MAX_COMMAND_OUTPUT]);
    assert!(!row.truncated);
    let mut observation = Observation::new(WorktreeId::random(), &[row.command]).unwrap();
    observation.snapshot.commands.first_mut().unwrap().state = SetupCommandState::Unknown;
    observation.finish(true);
    assert_eq!(observation.snapshot.state, SetupRunState::Unknown);
}

#[test]
fn an_empty_approved_setup_can_complete_but_an_oversized_plan_is_refused() {
    let mut empty = Observation::new(WorktreeId::random(), &[]).unwrap();
    empty.finish(true);
    assert_eq!(empty.snapshot.state, SetupRunState::Succeeded);
    let count = usize::try_from(MAX_SETUP_PROGRESS_COMMANDS)
        .unwrap()
        .saturating_add(1);
    let commands = vec!["echo bounded".parse().unwrap(); count];
    assert!(Observation::new(WorktreeId::random(), &commands).is_none());
}
