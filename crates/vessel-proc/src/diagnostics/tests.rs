//! Exercise complete diagnostic boundaries and actual fake process launches.
use super::*;
use crate::{InvalidGroup, SpawnError, Stream, spawn};
use std::error::Error as _;
use std::ffi::OsStr;
use std::time::Duration;
use vessel_testkit::Fake;

mod capture;
const SENTINEL: &str = "PRIVATE-CREATION-PROMPT-CANARY";
const DEADLINE: Duration = Duration::from_secs(30);

fn private() -> CommandSpec {
    CommandSpec::new(std::env::current_exe().unwrap().join(SENTINEL))
        .arg(SENTINEL)
        .cwd(SENTINEL)
        .env(SENTINEL, SENTINEL)
        .with_diagnostics(CommandDiagnostics::Redacted)
}
fn diagnostic_chain(error: &impl std::error::Error) -> String {
    let mut text = format!("{error:?} {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(&format!(" {cause:?} {cause}"));
        source = cause.source();
    }
    text
}

#[test]
fn private_debug_and_error_helpers_hide_data_without_changing_execution_fields() {
    let private = private();
    let standard = private
        .clone()
        .with_diagnostics(CommandDiagnostics::Standard);
    assert!(!format!("{private:?}").contains(SENTINEL));
    assert!(format!("{standard:?}").contains(SENTINEL));
    assert_eq!(
        CommandSpec::new("unused").diagnostic_policy(),
        CommandDiagnostics::Standard
    );
    let one = private.to_command();
    let two = standard.to_command();
    assert_eq!(one.get_program(), two.get_program());
    assert_eq!(
        one.get_args().collect::<Vec<_>>(),
        two.get_args().collect::<Vec<_>>()
    );
    assert_eq!(one.get_current_dir(), two.get_current_dir());
    assert_eq!(
        one.get_envs().collect::<Vec<_>>(),
        two.get_envs().collect::<Vec<_>>()
    );
    assert_eq!(one.get_args().collect::<Vec<_>>(), [OsStr::new(SENTINEL)]);
    let error = || io::Error::new(io::ErrorKind::PermissionDenied, SENTINEL);
    assert_eq!(standard.diagnostic_io_error(error()).to_string(), SENTINEL);
    let hidden = private.diagnostic_io_error(error());
    assert_eq!(hidden.kind(), io::ErrorKind::PermissionDenied);
    assert!(!diagnostic_chain(&hidden).contains(SENTINEL));
}

#[test]
fn every_process_spawn_error_redacts_the_program_and_backend_chain() {
    let spec = private();
    let program = || spec.program.clone();
    for error in [
        SpawnError::Start {
            program: program(),
            source: io::Error::other(SENTINEL),
        },
        SpawnError::MissingPipe {
            program: program(),
            stream: Stream::Stderr,
        },
        SpawnError::NoPid { program: program() },
        SpawnError::NotLeader {
            program: program(),
            source: InvalidGroup { pid: 0 },
        },
    ] {
        assert!(diagnostic_chain(&error).contains(SENTINEL));
        let hidden = error.with_diagnostics(&spec);
        assert!(!diagnostic_chain(&hidden).contains(SENTINEL));
        assert!(diagnostic_chain(&hidden).contains("<redacted>"));
    }
}

#[tokio::test]
async fn actual_failed_spawn_never_returns_the_private_program_or_prompt() {
    let error = spawn(&private()).unwrap_err();
    assert!(matches!(&error, SpawnError::Start { .. }));
    assert!(!diagnostic_chain(&error).contains(SENTINEL));
    assert!(error.source().is_some());
}

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
