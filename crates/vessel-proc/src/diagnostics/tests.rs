//! Exercise complete diagnostic boundaries and actual fake process launches.
use super::*;
use crate::{InvalidGroup, SpawnError, Stream, spawn};
use std::error::Error as _;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::sync::Arc;
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
        write!(text, " {cause:?} {cause}").unwrap();
        source = cause.source();
    }
    text
}

#[derive(Debug)]
struct TypedCause {
    marker: Arc<()>,
    detail: &'static str,
}
impl std::fmt::Display for TypedCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.detail)
    }
}
impl std::error::Error for TypedCause {}

#[test]
fn private_io_projection_owns_the_actual_typed_cause_without_a_public_escape() {
    let marker = Arc::new(());
    let original = io::Error::new(
        io::ErrorKind::PermissionDenied,
        TypedCause {
            marker: Arc::clone(&marker),
            detail: SENTINEL,
        },
    );
    let hidden = private().diagnostic_io_error(original);
    assert_eq!(hidden.kind(), io::ErrorKind::PermissionDenied);
    let owned = hidden
        .get_ref()
        .unwrap()
        .downcast_ref::<PrivateIoCause>()
        .unwrap();
    let original = owned
        ._original
        .get_ref()
        .unwrap()
        .downcast_ref::<TypedCause>()
        .unwrap();
    assert!(
        Arc::ptr_eq(&original.marker, &marker),
        "the actual cause must stay owned"
    );
    assert_eq!(Arc::strong_count(&marker), 2);
    assert!(!diagnostic_chain(&hidden).contains(SENTINEL));
    // These standard public io::Error escape hatches reveal only the safe
    // private wrapper as dyn Error, never the underlying io::Error or its cause.
    assert!(
        hidden
            .get_ref()
            .unwrap()
            .downcast_ref::<io::Error>()
            .is_none()
    );
    let inner = hidden.into_inner().unwrap();
    assert!(!format!("{inner:?} {inner}").contains(SENTINEL));
    assert!(inner.source().is_none());
    assert_eq!(Arc::strong_count(&marker), 2);
    drop(inner);
    assert_eq!(Arc::strong_count(&marker), 1);
}

#[test]
fn private_io_projection_keeps_original_os_evidence_and_standard_error_unchanged() {
    const OS_EVIDENCE: i32 = 17;
    let original = io::Error::from_raw_os_error(OS_EVIDENCE);
    let kind = original.kind();
    let hidden = private().diagnostic_io_error(original);
    let owned = hidden
        .get_ref()
        .unwrap()
        .downcast_ref::<PrivateIoCause>()
        .unwrap();
    assert_eq!(owned._original.raw_os_error(), Some(OS_EVIDENCE));
    assert_eq!(hidden.kind(), kind);
    assert!(hidden.source().is_none());
    let plain =
        CommandSpec::new("unused").diagnostic_io_error(io::Error::from_raw_os_error(OS_EVIDENCE));
    assert_eq!(plain.raw_os_error(), Some(OS_EVIDENCE));
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
