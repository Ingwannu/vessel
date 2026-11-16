use super::*;
use futures_util::FutureExt as _;
use std::{
    fs::OpenOptions,
    io::Write as _,
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Semaphore, oneshot};

const WAIT_BUDGET: Duration = Duration::from_secs(10);
const ONE_PERMIT: usize = 1;
const GATE_MESSAGES: usize = 1;

fn private_root() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("initially private candidate");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(root.path().join("helper"))
        .expect("owned helper bytes, never executed");
    file.write_all(b"synthetic artifact")
        .expect("fixture bytes");
    root
}
fn permit(semaphore: &Arc<Semaphore>) -> OwnedSemaphorePermit {
    Arc::clone(semaphore)
        .try_acquire_owned()
        .expect("already admitted observation")
}
fn deadline() -> Instant {
    Instant::now()
        .checked_add(file::OBSERVATION_BUDGET)
        .expect("test observation deadline")
}

#[tokio::test]
async fn successful_job_runs_actual_passes_and_settles_permit_before_reply() {
    let root = private_root();
    let parent = CancellationToken::new();
    let expected = tree::Pending::begin(root.path(), &parent)
        .expect("first pass")
        .finish()
        .expect("actual synchronous observation");
    let semaphore = Arc::new(Semaphore::new(ONE_PERMIT));
    let tasks = TaskTracker::new();
    let job = Observation::start(
        root.path().to_path_buf(),
        permit(&semaphore),
        &tasks,
        &parent,
    )
    .expect("tracked actual worker");
    let observed = tokio::time::timeout(WAIT_BUDGET, job.wait())
        .await
        .expect("finite wait")
        .expect("actual blocking filesystem observation");
    assert_eq!(observed, expected);
    assert_eq!(semaphore.available_permits(), ONE_PERMIT);
    assert!(
        !parent.is_cancelled(),
        "disposing a local waiter cannot cancel the node"
    );
    assert!(tasks.close());
    tokio::time::timeout(WAIT_BUDGET, tasks.wait())
        .await
        .expect("owned tracker settled");
}

#[tokio::test]
async fn losing_polled_waiter_keeps_actual_worker_and_permit_until_settlement() {
    let root = private_root();
    let parent = CancellationToken::new();
    let semaphore = Arc::new(Semaphore::new(ONE_PERMIT));
    let tasks = TaskTracker::new();
    let (started, seen) = oneshot::channel();
    let (release, held) = std::sync::mpsc::sync_channel(GATE_MESSAGES);
    let job = Observation::start_with_gate(
        root.path().to_path_buf(),
        permit(&semaphore),
        &tasks,
        &parent,
        deadline(),
        move || {
            started.send(()).expect("worker entered");
            held.recv().expect("held real blocking worker released");
        },
    )
    .expect("actual tracked closure");
    let child = job.cancellation.clone();
    tokio::time::timeout(WAIT_BUDGET, seen)
        .await
        .expect("worker started")
        .expect("entry receipt");
    assert!(
        job.wait().now_or_never().is_none(),
        "cancel an actually polled wait"
    );
    assert!(child.is_cancelled());
    assert!(!parent.is_cancelled());
    assert_eq!(
        semaphore.available_permits(),
        0,
        "caller loss cannot free active I/O admission"
    );
    assert_eq!(tasks.len(), ONE_PERMIT);
    assert!(tasks.close());
    assert!(
        tasks.wait().now_or_never().is_none(),
        "teardown must still wait for owned work"
    );
    release.send(()).expect("release the owned worker");
    tokio::time::timeout(WAIT_BUDGET, tasks.wait())
        .await
        .expect("real settlement, not waiter drop");
    assert_eq!(semaphore.available_permits(), ONE_PERMIT);
    assert_eq!(tasks.len(), 0);
}

#[tokio::test]
async fn captured_expired_deadline_is_not_restarted_at_worker_entry() {
    let parent = CancellationToken::new();
    let semaphore = Arc::new(Semaphore::new(ONE_PERMIT));
    let tasks = TaskTracker::new();
    let job = Observation::start_with_gate(
        PathBuf::from("PRIVATE-NEVER-OPENED"),
        permit(&semaphore),
        &tasks,
        &parent,
        Instant::now(),
        || {},
    )
    .expect("tracked worker checks captured deadline");
    let result = tokio::time::timeout(WAIT_BUDGET, job.wait())
        .await
        .expect("bounded deadline refusal");
    assert!(matches!(
        result,
        Err(Failure::Observation(tree::Failure::File(
            file::Failure::Expired
        )))
    ));
    assert_eq!(semaphore.available_permits(), ONE_PERMIT);
    assert!(tasks.close());
    tokio::time::timeout(WAIT_BUDGET, tasks.wait())
        .await
        .expect("worker settled");
}

#[test]
fn missing_runtime_refuses_before_spawning_or_reading() {
    let semaphore = Arc::new(Semaphore::new(ONE_PERMIT));
    let tasks = TaskTracker::new();
    assert!(matches!(
        Observation::start(
            PathBuf::from("PRIVATE-NEVER-OPENED"),
            permit(&semaphore),
            &tasks,
            &CancellationToken::new()
        ),
        Err(Failure::Runtime(_))
    ));
    assert_eq!(semaphore.available_permits(), ONE_PERMIT);
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn already_cancelled_parent_refuses_before_tracking_work() {
    let semaphore = Arc::new(Semaphore::new(ONE_PERMIT));
    let tasks = TaskTracker::new();
    let parent = CancellationToken::new();
    parent.cancel();
    assert!(matches!(
        Observation::start(
            PathBuf::from("PRIVATE-NEVER-OPENED"),
            permit(&semaphore),
            &tasks,
            &parent
        ),
        Err(Failure::Observation(tree::Failure::File(
            file::Failure::Cancelled
        )))
    ));
    assert_eq!(semaphore.available_permits(), ONE_PERMIT);
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn worker_panic_retains_concrete_cause_but_seals_public_diagnostics() {
    let semaphore = Arc::new(Semaphore::new(ONE_PERMIT));
    let tasks = TaskTracker::new();
    let job = Observation::start_with_gate(
        PathBuf::from("PRIVATE-NEVER-OPENED"),
        permit(&semaphore),
        &tasks,
        &CancellationToken::new(),
        deadline(),
        || panic!("PRIVATE-OPERATION-CANARY"),
    )
    .expect("owned synthetic panic");
    let error = tokio::time::timeout(WAIT_BUDGET, job.wait())
        .await
        .expect("bounded panic receipt")
        .expect_err("actual join failure");
    let mut source: Option<&(dyn Error + 'static)> = Some(&error);
    while let Some(error) = source {
        assert!(!format!("{error:?} {error}").contains("PRIVATE-OPERATION-CANARY"));
        source = error.source();
    }
    assert!(matches!(error, Failure::Worker(cause) if cause.0.is_panic()));
    assert_eq!(semaphore.available_permits(), ONE_PERMIT);
    assert!(tasks.close());
    tokio::time::timeout(WAIT_BUDGET, tasks.wait())
        .await
        .expect("panicked worker tracked to completion");
}
