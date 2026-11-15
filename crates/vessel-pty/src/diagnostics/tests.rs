//! The real synchronous spawn boundary, logger bridge and thread-local scope.
use crate::{PtyProcess, WindowSize};
use std::{process::Command, sync::OnceLock, thread::ThreadId};
use vessel_proc::{CommandDiagnostics, CommandSpec};

const CHILD: &str = "VESSEL_PRIVATE_PTY_LOG_CANARY";
const SENTINEL: &str = "PRIVATE-WINDOWS-CREATEPROCESS-CANARY";
static SPAWN_THREAD: OnceLock<ThreadId> = OnceLock::new();

// Called immediately before the actual portable-pty spawn in the same frame.
// Moving that backend call to a worker/task also moves this assertion and fails.
pub(crate) fn backend_probe() {
    if let Some(expected) = SPAWN_THREAD.get() {
        assert_eq!(*expected, std::thread::current().id());
        log::error!(target: "portable_pty::win::psuedocon", "{SENTINEL}");
        tracing::error!(target: "portable_pty::win::psuedocon", "{SENTINEL}");
        log::error!(target: "vessel_pty_canary", "other-backend-record-retained");
        assert_eq!(
            tracing::Span::current().metadata().unwrap().name(),
            "spawn-parent"
        );
    }
}

#[test]
fn private_pty_backend_errors_use_the_real_bridge_and_restore_the_parent() {
    if let Some(mode) = std::env::var_os(CHILD) {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .init();
        SPAWN_THREAD.set(std::thread::current().id()).unwrap();
        let private = mode == "private";
        let policy = if private {
            CommandDiagnostics::Redacted
        } else {
            CommandDiagnostics::Standard
        };
        let spec = CommandSpec::new(std::env::current_exe().unwrap().join(SENTINEL))
            .arg(SENTINEL)
            .with_diagnostics(policy);
        let _parent = tracing::info_span!("spawn-parent").entered();
        let error = PtyProcess::spawn(&spec, WindowSize { cols: 80, rows: 24 }).unwrap_err();
        assert_eq!(!format!("{error:?} {error}").contains(SENTINEL), private);
        log::error!(target: "portable_pty", "outside-scope-record-retained");
        return;
    }
    for mode in ["private", "standard"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "diagnostics::tests::private_pty_backend_errors_use_the_real_bridge_and_restore_the_parent",
                "--nocapture",
            ])
            .env(CHILD, mode)
            .env("RUST_LOG", "trace")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(stderr.contains(SENTINEL), mode == "standard");
        assert!(stderr.contains("other-backend-record-retained"));
        assert!(stderr.contains("outside-scope-record-retained"));
        assert!(stderr.contains("spawn-parent"));
    }
}
