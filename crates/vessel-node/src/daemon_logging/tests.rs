//! Run the actual binary initializer in a child: no shared environment mutation.
use std::process::Command;

const CHILD: &str = "VESSEL_LOG_FILTER_CANARY";
const SENTINEL: &str = "PRIVATE-INHERITED-ENVIRONMENT-CANARY";

#[test]
fn portable_pty_trace_is_capped_after_the_user_filter_and_log_bridge() {
    if std::env::var_os(CHILD).is_some() {
        super::init(false);
        tracing::trace!(target: "portable_pty", "{SENTINEL}");
        tracing::trace!(target: "portable_pty::cmdbuilder", "{SENTINEL}");
        log::trace!(target: "portable_pty", "{SENTINEL}");
        log::trace!(target: "portable_pty::cmdbuilder", "{SENTINEL}");
        log::debug!(target: "portable_pty::cmdbuilder", "{SENTINEL}");
        log::warn!(target: "portable_pty::cmdbuilder", "backend-warning-retained");
        tracing::trace!(target: "vessel_filter_canary", "other-trace-retained");
        log::trace!(target: "vessel_filter_canary", "other-log-trace-retained");
        return;
    }
    for user in [
        "trace",
        "trace,portable_pty=trace,portable_pty::cmdbuilder=trace",
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "daemon_logging::tests::portable_pty_trace_is_capped_after_the_user_filter_and_log_bridge",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("RUST_LOG", user)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains(SENTINEL), "private backend record escaped");
        assert!(stderr.contains("backend-warning-retained"));
        assert!(stderr.contains("other-trace-retained"));
        assert!(stderr.contains("other-log-trace-retained"));
    }
}
