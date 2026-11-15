//! A blocked FIFO regression runs in one owned copy of this test executable.
//!
//! [Decision Log]
//! - Why: a timed-out thread cannot be killed or safely detached. A process
//!   can be terminated and explicitly reaped before the parent reports failure.
//! - Reuse: the existing node/proc self-test-executable pattern, without
//!   testkit::within (which prebuilds unrelated fake harnesses).
//! - Constraint: retain the original three-second safety net and real FIFO
//!   refusal. No product flag, file permission or assertion is weakened.

use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use super::super::{Directory, Error, Name, Purpose};

const FIFO_INSPECTION_BUDGET: Duration = Duration::from_secs(3);
const CHILD_ROOT: &str = "VESSEL_PRIVATE_FIFO_TEST_ROOT";
const CHILD_MODE: &str = "VESSEL_PRIVATE_FIFO_TEST_MODE";
const CHILD_TEST: &str = "private_files::tests::fifo::child_probe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Completed,
    TimedOut,
}

#[derive(Clone, Copy)]
enum Probe {
    Inspect,
    DeliberatelyBlock,
}

pub(super) async fn refuses_without_leaving_test_work() {
    let (phase, status) = run(Probe::Inspect).await;
    // Both assertions happen only after the actual child has been reaped.
    assert_eq!(
        phase,
        Phase::Completed,
        "FIFO inspection waited for a writer"
    );
    assert!(status.success(), "the child did not refuse the real FIFO");
}

/// Exercise the failure cleanup with an actually blocked FIFO open, rather
/// than trusting kill_on_drop or a dropped task/thread handle as a receipt.
#[tokio::test]
async fn the_deadline_terminates_and_reaps_the_actual_blocked_probe() {
    let (phase, status) = run(Probe::DeliberatelyBlock).await;
    assert_eq!(phase, Phase::TimedOut);
    assert!(!status.success(), "the blocked probe must be terminated");
}

async fn run(probe: Probe) -> (Phase, ExitStatus) {
    let temp = tempfile::tempdir().expect("private FIFO fixture");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        temp.path().join("input"),
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .expect("fixture FIFO");
    let mode = match probe {
        Probe::Inspect => "inspect",
        Probe::DeliberatelyBlock => "block",
    };
    let mut child = tokio::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", CHILD_TEST, "--test-threads=1"])
        .env_clear()
        .env(CHILD_ROOT, temp.path())
        .env(CHILD_MODE, mode)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn this test executable only");
    let outcome = tokio::time::timeout(FIFO_INSPECTION_BUDGET, child.wait()).await;
    let settled = match outcome {
        Ok(Ok(status)) => (Phase::Completed, status),
        Err(_) => (Phase::TimedOut, terminate_and_reap(&mut child).await),
        Ok(Err(error)) => {
            let _status = terminate_and_reap(&mut child).await;
            panic!("FIFO inspection wait failed after cleanup: {error}");
        }
    };
    assert!(
        child.id().is_none(),
        "actual child was reaped before assertions"
    );
    assert!(
        temp.path().join("entered").is_file(),
        "the exact child test ran"
    );
    settled
}

async fn terminate_and_reap(child: &mut tokio::process::Child) -> ExitStatus {
    let killed = child.start_kill();
    // Even if signalling fails, attempt the actual wait before reporting it.
    let reaped = child.wait().await;
    assert!(
        killed.is_ok(),
        "terminate the owned fixture process: {killed:?}"
    );
    reaped.expect("reap the owned fixture process")
}

#[test]
fn child_probe() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let root = Path::new(&root);
    std::fs::write(root.join("entered"), b"entered").expect("bounded execution marker");
    match std::env::var(CHILD_MODE)
        .expect("parent-selected mode")
        .as_str()
    {
        "inspect" => {
            let directory = Directory::open(root).expect("trusted fixture anchor");
            assert!(matches!(
                directory.read(Name::parse("input").unwrap(), Purpose::Credential),
                Err(Error::UnsafeTarget)
            ));
        }
        "block" => {
            let _file = std::fs::File::open(root.join("input"));
            panic!("a FIFO without a writer must remain blocked until cleanup");
        }
        other => panic!("unknown test-only child mode: {other}"),
    }
}
