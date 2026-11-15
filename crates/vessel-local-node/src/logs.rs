//! A bounded stderr actor serves the last log lines only when explicitly asked.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt as _};
use tokio::sync::{mpsc, oneshot};

/// Paseo's managed-daemon log viewer requests its last one hundred lines.
const MAX_LOG_LINES: usize = 100;
/// Byte bound also protects the viewer from a single enormous line.
pub(crate) const MAX_LOG_BYTES: usize = 128 * 1024;
const READ_CHUNK_BYTES: usize = 4096;
const LOG_REQUESTS: usize = 2;
const LOG_READ_DEADLINE: Duration = Duration::from_secs(2);

/// Diagnostic text is opaque; it is never parsed into process-control decisions.
#[derive(Clone)]
pub struct LogText(Arc<str>);

impl std::fmt::Debug for LogText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogText")
            .field("bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}

impl LogText {
    /// Render or copy only at an explicit local log-view action.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Reading logs cannot wait forever when the launcher runtime has stopped.
#[derive(Debug, thiserror::Error)]
pub enum LogError {
    /// Keep stderr I/O failure distinct from a successfully exhausted stream.
    #[error("local node stderr read failed")]
    Read(#[source] Arc<std::io::Error>),
    /// The bounded request queue is full or its collector has ended.
    #[error("local node log collector is unavailable")]
    Unavailable,
    /// A live collector did not answer within its named deadline.
    #[error("local node log read timed out")]
    Deadline(#[source] tokio::time::error::Elapsed),
}

struct Reader {
    requests: mpsc::Sender<oneshot::Sender<Result<LogText, LogError>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Clones share only an actor handle; stderr and its bounded ring have one owner.
#[derive(Clone)]
pub struct LogReader(Arc<Reader>);

impl std::fmt::Debug for LogReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogReader").finish_non_exhaustive()
    }
}

impl LogReader {
    pub(crate) fn collect(stderr: impl AsyncRead + Unpin + Send + 'static) -> Self {
        let (requests, receiver) = mpsc::channel(LOG_REQUESTS);
        let task = tokio::spawn(collect(stderr, receiver));
        Self(Arc::new(Reader { requests, task }))
    }

    /// Read the bounded tail; no log contents enter tracing or error messages.
    ///
    /// # Errors
    /// Collector unavailable, full queue, or its response deadline elapsed.
    pub async fn read(&self) -> Result<LogText, LogError> {
        let (answer, response) = oneshot::channel();
        self.0
            .requests
            .try_send(answer)
            .map_err(|_refused| LogError::Unavailable)?;
        tokio::time::timeout(LOG_READ_DEADLINE, response)
            .await
            .map_err(LogError::Deadline)?
            .map_err(|_closed| LogError::Unavailable)?
    }
}

enum Stream {
    Open,
    Closed,
}

async fn collect(
    mut stderr: impl AsyncRead + Unpin,
    mut requests: mpsc::Receiver<oneshot::Sender<Result<LogText, LogError>>>,
) {
    let mut ring = VecDeque::with_capacity(MAX_LOG_BYTES);
    let mut chunk = [0_u8; READ_CHUNK_BYTES];
    let mut stream = Stream::Open;
    let mut failure = None;
    loop {
        tokio::select! {
            read = stderr.read(&mut chunk), if matches!(stream, Stream::Open) => {
                match read {
                    Ok(0) => stream = Stream::Closed,
                    Err(error) => { stream = Stream::Closed; failure = Some(Arc::new(error)); },
                    Ok(count) => {
                        if let Some(bytes) = chunk.get(..count) {
                            for byte in bytes {
                                if ring.len() == MAX_LOG_BYTES { let _old = ring.pop_front(); }
                                ring.push_back(*byte);
                            }
                        }
                    }
                }
            }
            request = requests.recv() => {
                let Some(answer) = request else { return; };
                if let Some(error) = &failure {
                    let _closed = answer.send(Err(LogError::Read(Arc::clone(error))));
                    continue;
                }
                let bytes: Vec<_> = ring.iter().copied().collect();
                let _closed = answer.send(Ok(tail(&bytes)));
            }
        }
    }
}

pub(crate) fn tail(bytes: &[u8]) -> LogText {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<_> = text.lines().rev().take(MAX_LOG_LINES).collect();
    let tail = lines.into_iter().rev().collect::<Vec<_>>().join("\n");
    LogText(tail.into())
}
