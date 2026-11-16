//! Private errors include no backend text; actual PTY launch uses the safe log label.
use super::*;
use crate::{PtyProcess, WindowSize};
use std::fmt::Write as _;
use std::{error::Error as _, io, time::Duration};
use vessel_proc::{CommandDiagnostics, CommandSpec, InvalidGroup};
use vessel_testkit::Fake;

mod capture;
const SENTINEL: &str = "PRIVATE-CREATION-PROMPT-CANARY";
const DEADLINE: Duration = Duration::from_secs(30);
const SIZE: WindowSize = WindowSize { cols: 80, rows: 24 };

fn chain(error: &PtyError) -> String {
    let mut text = format!("{error:?} {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        write!(text, " {cause:?} {cause}").unwrap();
        source = cause.source();
    }
    text
}

#[test]
fn all_pty_error_variants_keep_safe_shapes_and_remove_private_details() {
    let spec = CommandSpec::new(SENTINEL).with_diagnostics(CommandDiagnostics::Redacted);
    for error in [
        PtyError::Open {
            message: SENTINEL.to_owned(),
        },
        PtyError::Spawn {
            program: SENTINEL.to_owned(),
            message: SENTINEL.to_owned(),
        },
        PtyError::NoPid {
            program: SENTINEL.to_owned(),
        },
        PtyError::Group {
            program: SENTINEL.to_owned(),
            source: InvalidGroup { pid: 0 },
        },
        PtyError::Attach {
            message: SENTINEL.to_owned(),
        },
        PtyError::Thread(io::Error::other(SENTINEL)),
    ] {
        assert!(chain(&error).contains(SENTINEL));
        let hidden = error.with_diagnostics(&spec);
        assert!(!chain(&hidden).contains(SENTINEL));
    }
    let plain = CommandSpec::new("unchanged");
    let retained = PtyError::Spawn {
        program: SENTINEL.to_owned(),
        message: SENTINEL.to_owned(),
    }
    .with_diagnostics(&plain);
    assert!(chain(&retained).contains(SENTINEL));
}

#[test]
fn actual_failed_pty_spawn_does_not_report_private_program_or_arguments() {
    let spec = CommandSpec::new(std::env::current_exe().unwrap().join(SENTINEL))
        .arg(SENTINEL)
        .with_diagnostics(CommandDiagnostics::Redacted);
    let error = PtyProcess::spawn(&spec, SIZE).unwrap_err();
    assert!(!chain(&error).contains(SENTINEL));
}

#[tokio::test]
async fn actual_pty_spawn_log_is_private_and_the_program_still_runs() {
    vessel_testkit::within(DEADLINE, async {
        let program = Fake::Tui.path().unwrap();
        let spec = CommandSpec::new(&program)
            .args(["--version", SENTINEL])
            .with_diagnostics(CommandDiagnostics::Redacted);
        let capture = capture::Capture::default();
        let (mut process, mut output) =
            tracing::subscriber::with_default(capture.clone(), || PtyProcess::spawn(&spec, SIZE))
                .unwrap();
        assert_eq!(process.wait().await, Some(0));
        let mut actual = Vec::new();
        while let Some(bytes) = output.next().await {
            actual.extend_from_slice(&bytes);
        }
        assert!(String::from_utf8_lossy(&actual).contains("fake-tui"));
        let events = capture.text();
        assert!(events.contains("pty spawned"), "actual spawn event missing");
        assert!(events.contains("<redacted>"));
        assert!(!events.contains(SENTINEL));
        assert!(!events.contains(&program.display().to_string()));
    })
    .await
    .unwrap();
}
