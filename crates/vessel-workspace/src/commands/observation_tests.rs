//! These assertions depend on runner boundaries, never printable markers.

use super::*;

fn invocation<'a>(folder: &'a Path, log: &'a Path, cancel: CancellationToken) -> Run<'a> {
    Run {
        dir: folder,
        env: Vec::new(),
        removed: &[],
        log,
        scratch: folder,
        limit: SETUP_LIMIT,
        cancel,
    }
}

#[tokio::test]
async fn forged_log_markers_do_not_create_commands_and_failure_stops_the_list() {
    let folder = tempfile::tempdir().unwrap();
    let log = folder.path().join("setup.log");
    let written = tokio::sync::watch::channel(0).0;
    let commands = ["printf '$ fake command\\n'", "exit 7", "touch never"]
        .map(|text| text.parse::<ProjectCommand>().unwrap());
    let run = invocation(folder.path(), &log, CancellationToken::new());
    let mut starts = Vec::new();
    let mut finishes = Vec::new();
    let mut output = Vec::new();
    let result = run_observed(&commands, &run, &written, |event| match event {
        CommandEvent::Started { index, command } => {
            starts.push((index.get(), command.as_str().to_owned()));
        }
        CommandEvent::Output { index, bytes } => {
            output.push((index.get(), bytes.to_vec()));
        }
        CommandEvent::Finished {
            index,
            elapsed,
            outcome,
        } => {
            finishes.push((index.get(), elapsed, outcome));
        }
    })
    .await
    .unwrap();
    assert_eq!(starts.len(), 2);
    assert_eq!(
        starts.first(),
        Some(&(0, commands.first().unwrap().as_str().to_owned()))
    );
    assert_eq!(
        finishes
            .iter()
            .map(|(index, _, outcome)| (*index, *outcome))
            .collect::<Vec<_>>(),
        [
            (0, CommandOutcome::Exited(Some(0))),
            (1, CommandOutcome::Exited(Some(7)))
        ]
    );
    assert!(output.iter().all(|(index, _)| *index == 0));
    let bytes = output
        .into_iter()
        .flat_map(|(_, bytes)| bytes)
        .collect::<Vec<_>>();
    assert_eq!(bytes, b"$ fake command\n");
    assert_eq!(
        result,
        Some(SetupFailure::Exit {
            command: 1,
            code: Some(7)
        })
    );
    assert!(!folder.path().join("never").exists());
}

#[tokio::test]
async fn cancelling_a_running_command_reports_cancelled_and_never_starts_the_next() {
    let folder = tempfile::tempdir().unwrap();
    let log = folder.path().join("setup.log");
    let written = tokio::sync::watch::channel(0).0;
    let cancel = CancellationToken::new();
    let run = invocation(folder.path(), &log, cancel.clone());
    let commands = ["printf ready; sleep 30", "touch never"]
        .map(|text| text.parse::<ProjectCommand>().unwrap());
    let mut starts = Vec::new();
    let mut finished = Vec::new();
    let result = run_observed(&commands, &run, &written, |event| match event {
        CommandEvent::Started { index, command: _ } => starts.push(index.get()),
        CommandEvent::Output { index: _, bytes: _ } => cancel.cancel(),
        CommandEvent::Finished {
            index: _,
            elapsed: _,
            outcome,
        } => finished.push(outcome),
    })
    .await
    .unwrap();
    assert_eq!(starts, [0]);
    assert_eq!(finished, [CommandOutcome::Cancelled]);
    assert_eq!(
        result,
        Some(SetupFailure::Exit {
            command: 0,
            code: None
        })
    );
    assert!(!folder.path().join("never").exists());
}

#[tokio::test]
async fn a_pre_cancelled_run_never_spawns_and_observation_survives_a_full_global_log() {
    let folder = tempfile::tempdir().unwrap();
    let log = folder.path().join("setup.log");
    let written = tokio::sync::watch::channel(0).0;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let run = invocation(folder.path(), &log, cancel);
    let commands = ["touch never".parse::<ProjectCommand>().unwrap()];
    let mut finished = Vec::new();
    let _failure = run_observed(&commands, &run, &written, |event| {
        if let CommandEvent::Finished {
            index: _,
            elapsed: _,
            outcome,
        } = event
        {
            finished.push(outcome);
        }
    })
    .await
    .unwrap();
    assert_eq!(finished, [CommandOutcome::Cancelled]);
    assert!(!folder.path().join("never").exists());

    let file = std::fs::File::create(&log).unwrap();
    file.set_len(MAX_LOG).unwrap();
    let run = invocation(folder.path(), &log, CancellationToken::new());
    let commands = ["printf observed".parse::<ProjectCommand>().unwrap()];
    let mut output = Vec::new();
    let result = run_observed(&commands, &run, &written, |event| {
        if let CommandEvent::Output { index: _, bytes } = event {
            output.extend_from_slice(bytes);
        }
    })
    .await
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(output, b"observed");
    assert_eq!(std::fs::metadata(&log).unwrap().len(), MAX_LOG);
}
