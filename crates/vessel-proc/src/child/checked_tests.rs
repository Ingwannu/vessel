//! Only this test executable is launched; no nested fake/agent build or network.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncWriteExt;

use super::*;

const PROBE: &str = "VESSEL_CHECKED_WAIT_PROBE";
const PROBE_TEST: &str = "child::checked_tests::child_probe";
const CANCEL_WAIT_BUDGET: Duration = Duration::from_millis(20);
const REAP_WAIT_BUDGET: Duration = Duration::from_secs(10);

fn command(wait: bool) -> CommandSpec {
    let mut command = CommandSpec::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", PROBE_TEST])
        .env_clear();
    if wait {
        command = command.env(PROBE, "wait");
    }
    if cfg!(windows) {
        command = command.env(
            "SystemRoot",
            std::env::var_os("SystemRoot").expect("platform root"),
        );
    }
    command
}

#[test]
fn child_probe() {
    if std::env::var_os(PROBE).as_deref() == Some(std::ffi::OsStr::new("wait")) {
        use std::io::Read;
        let mut byte = [0; 1];
        std::io::stdin()
            .lock()
            .read_exact(&mut byte)
            .expect("controlled fixture stdin");
    }
}

#[tokio::test]
async fn wait_checked_reuses_legacy_wait_outcome_and_revokes_the_existing_observation() {
    let Spawned {
        mut child,
        stdin: _stdin,
        stdout: _stdout,
        stderr: _stderr,
    } = spawn(&command(false)).expect("spawn self");
    let lifetime = child.lifetime();
    assert_eq!(child.wait().await.expect("legacy wait"), Some(0));
    assert!(!lifetime.is_live());
    let first = child.wait_checked().await.expect("first receipt");
    let second = child.wait_checked().await.expect("cached receipt");
    assert_eq!(first, second);
    assert_eq!(first.code(), Some(0));
    assert!(first.run().same_run(&lifetime));
}

#[tokio::test]
async fn wait_checked_returns_the_first_sweep_failure_after_legacy_leader_success() {
    let Spawned {
        mut child,
        stdin: _stdin,
        stdout: _stdout,
        stderr: _stderr,
    } = spawn(&command(false)).expect("spawn self");
    assert_eq!(child.wait().await.expect("legacy wait"), Some(0));
    // Inject only the syscall outcome in this one owned test instance, after
    // reaping. No process/permission is changed to produce a real EPERM.
    let mut first = FirstSweep::default();
    let refused = first
        .record(|| {
            Err(SignalError::from(std::io::Error::from(
                std::io::ErrorKind::PermissionDenied,
            )))
        })
        .expect_err("synthetic OS refusal");
    child.first_sweep = first;
    for _ in 0..2 {
        let CheckedWaitError::Sweep(cause) =
            child.wait_checked().await.expect_err("refusal retained")
        else {
            panic!("sweep cause");
        };
        assert!(Arc::ptr_eq(&refused, &cause));
        assert!(cause.is_refused());
    }
    assert_eq!(
        child.wait().await.expect("legacy behavior unchanged"),
        Some(0)
    );
}

#[tokio::test]
async fn wait_checked_never_retries_a_reaped_group_without_recorded_evidence() {
    let Spawned {
        mut child,
        stdin: _stdin,
        stdout: _stdout,
        stderr: _stderr,
    } = spawn(&command(false)).expect("spawn self");
    let _code = child.wait().await.expect("reaped");
    child.first_sweep = FirstSweep::default();
    assert!(matches!(
        child.wait_checked().await,
        Err(CheckedWaitError::Unrecorded)
    ));
}

#[tokio::test]
async fn cancelling_wait_checked_preserves_owned_child_and_later_reap() {
    let Spawned {
        mut child,
        mut stdin,
        stdout: _stdout,
        stderr: _stderr,
    } = spawn(&command(true)).expect("spawn controlled child");
    assert!(
        tokio::time::timeout(CANCEL_WAIT_BUDGET, child.wait_checked())
            .await
            .is_err()
    );
    stdin.write_all(b"x").await.expect("finish child");
    let receipt = tokio::time::timeout(REAP_WAIT_BUDGET, child.wait_checked())
        .await
        .expect("reap deadline")
        .expect("checked reap");
    assert_eq!(receipt.code(), Some(0));
}
