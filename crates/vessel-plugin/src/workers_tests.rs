//! Deterministic cleanup-before-reader and partial-reader cancellation cases.
use super::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{io::ReadBuf, sync::oneshot};
use vessel_proto::plugins::LogTruncation;

#[cfg(unix)]
const REAP_FIXTURE_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);
#[cfg(unix)]
const STDERR_FIXTURE: &str = "VESSEL_PLUGIN_HELD_STDERR_FIXTURE";
const FINAL_DIAGNOSTIC: &str = "last unterminated diagnostic";

#[cfg(unix)]
#[test]
fn stderr_exit_fixture() {
    use std::io::Write as _;
    if std::env::var_os(STDERR_FIXTURE).is_some() {
        std::io::stderr()
            .write_all(FINAL_DIAGNOSTIC.as_bytes())
            .expect("fixture stderr");
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn checked_leader_cleanup_precedes_the_first_stderr_turn_without_losing_final_bytes() {
    let directory = tempfile::tempdir().expect("isolated fixture");
    let command = vessel_proc::CommandSpec::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("workers::tests::stderr_exit_fixture")
        .arg("--nocapture")
        .cwd(directory.path())
        .env_clear()
        .env(STDERR_FIXTURE, "1");
    let vessel_proc::Spawned {
        mut child,
        stdin,
        stdout: _stdout,
        stderr,
    } = vessel_proc::spawn(&command).expect("owned fixture");
    drop(stdin);
    let logs = SharedLogs::default();
    let retained = logs.clone();
    let (release, held) = oneshot::channel();
    let mut workers = Workers::new();
    let _reader = workers.tasks.spawn(async move {
        held.await.expect("explicit held reader release");
        read_stderr(stderr, retained).await
    });
    let receipt = tokio::time::timeout(REAP_FIXTURE_BUDGET, child.wait_checked())
        .await
        .expect("leader budget")
        .expect("checked cleanup");
    assert_eq!(receipt.code(), Some(0));
    assert!(logs.snapshot().expect("tail before reader").is_empty());
    // The single-thread test cannot run the reader between this release and
    // drain's first await. An immediate abort-all loses the pipe deterministically.
    release.send(()).expect("release after real leader cleanup");
    workers.drain().await.expect("bounded EOF drain");
    let entries = logs.snapshot().expect("retained final diagnostic");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message.as_str(), FINAL_DIAGNOSTIC);
    assert_eq!(entries[0].truncation, LogTruncation::Complete);
    assert!(workers.tasks.is_empty());
}

/// The second poll is an acknowledgement that the prior read returned to the
/// parser. It then keeps the stream open without supplying EOF or another byte.
struct RetainedPipe {
    prefix: Option<&'static [u8]>,
    consumed: Option<oneshot::Sender<()>>,
}
impl AsyncRead for RetainedPipe {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let pipe = self.get_mut();
        if let Some(prefix) = pipe.prefix.take() {
            buffer.put_slice(prefix);
            return Poll::Ready(Ok(()));
        }
        if let Some(consumed) = pipe.consumed.take() {
            drop(consumed.send(()));
        }
        Poll::Pending
    }
}

#[tokio::test(start_paused = true)]
async fn retained_pipe_times_out_joins_reader_and_marks_the_captured_partial_line() {
    let (consumed, parsed) = oneshot::channel();
    let input = RetainedPipe {
        prefix: Some(FINAL_DIAGNOSTIC.as_bytes()),
        consumed: Some(consumed),
    };
    let logs = SharedLogs::default();
    let mut workers = Workers::new();
    let _reader = workers.tasks.spawn(read_stderr(input, logs.clone()));
    parsed
        .await
        .expect("reader has fed the partial bytes before holding the pipe");
    assert!(logs.snapshot().expect("no newline yet").is_empty());
    assert!(matches!(
        workers.drain().await,
        Err(RuntimeError::StderrDrainTimeout)
    ));
    assert!(workers.tasks.is_empty());
    let entries = logs.snapshot().expect("cancelled reader retained prefix");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message.as_str(), FINAL_DIAGNOSTIC);
    assert_eq!(entries[0].truncation, LogTruncation::PrefixRetained);
}
