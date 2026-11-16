//! A fresh ignored-SIGCHLD embedding must not invalidate retained PID evidence.

use super::*;
use std::process::{Command as StdCommand, Stdio};

const FIXTURE: &str = "VESSEL_IGNORED_CHILD_REAPING_FIXTURE";
const VERIFIED: &str = "managed-child-reaping-verified";
const MAX_FIXTURE_BYTES: u64 = 8 * 1024;
const ECHILD: i32 = 10;

fn short_child(root: &std::path::Path) -> std::process::Child {
    StdCommand::new("/bin/sh")
        .args(["-c", "exit 0"])
        .env_clear()
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn inherited_ignored_sigchld_fixture() {
    if std::env::var_os(FIXTURE).is_none() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        // No explicit child-signal registration yet: prove the inherited ignore
        // really auto-reaps, rather than trusting the shell command's spelling.
        let mut baseline = short_child(root.path());
        let baseline_pid = baseline.id();
        let baseline_wait = tokio::task::spawn_blocking(move || baseline.wait());
        let error = step("ignored child original wait", baseline_wait)
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(ECHILD));
        assert!(!std::fs::exists(format!("/proc/{baseline_pid}")).unwrap());

        let mut retained_registration = register_reaping().unwrap();
        let mut retained = short_child(root.path());
        step(
            "registered child exit notification",
            retained_registration.recv(),
        )
        .await
        .expect("child signal stream ended");
        // This isolated process has no other child. The actual SIGCHLD event
        // orders one read-only snapshot; no sleep or polling supplies evidence.
        let path = format!("/proc/{}/stat", retained.id());
        let stat = std::fs::read_to_string(&path).unwrap();
        let (_, fields) = stat.rsplit_once(')').unwrap();
        assert_eq!(fields.split_whitespace().next(), Some("Z"));
        let retained_wait = tokio::task::spawn_blocking(move || retained.wait());
        assert!(
            step("registered child original wait", retained_wait)
                .await
                .unwrap()
                .unwrap()
                .success()
        );
        assert!(!std::fs::exists(path).unwrap());
        writeln!(std::io::stdout().lock(), "{VERIFIED}").unwrap();
    });
}

#[tokio::test]
async fn ignored_sigchld_is_replaced_before_pid_based_ownership_evidence() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let command = CommandSpec::new("/bin/sh")
            .args(["-c", "trap '' CHLD; exec \"$@\"", "managed-reaping-fixture"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "launch::owned::tests::reaping::inherited_ignored_sigchld_fixture",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env(FIXTURE, "1")
            .cwd(root.path());
        let Spawned {
            mut child,
            stdin,
            stdout,
            mut stderr,
        } = spawn(&command).unwrap();
        drop(stdin);
        // These futures remain in this scope: failure drops the retained owner
        // and both readers, rather than detaching output tasks from cleanup.
        let output = async move {
            let mut bytes = Vec::new();
            let _read = stdout
                .take(MAX_FIXTURE_BYTES)
                .read_to_end(&mut bytes)
                .await
                .unwrap();
            bytes
        };
        let errors = async move {
            let _drained = tokio::io::copy(&mut stderr, &mut tokio::io::sink())
                .await
                .unwrap();
        };
        let (exit, output, ()) = tokio::join!(
            step("ignored-SIGCHLD fixture checked exit", child.wait_checked()),
            step("ignored-SIGCHLD fixture stdout ACK", output),
            step("ignored-SIGCHLD fixture stderr EOF", errors),
        );
        let exit = exit.unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(exit.code(), Some(0));
        assert!(
            output.contains(VERIFIED),
            "the exact fixture assertions did not run"
        );
    })
    .await
    .unwrap();
}
