//! Each pipe has one cancellation owner; selecting another actor event cannot
//! discard a half-read header or a half-written frame.

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::mpsc,
    task::{AbortHandle, JoinError, JoinSet},
};
use vessel_proto::plugins::{HelperToNode, NodeToHelper};

use crate::{
    log_tail::StderrLines, runtime_error::RuntimeError, runtime_types::REQUEST_DEADLINE,
    shared_logs::SharedLogs, wire,
};

const STDERR_READ_BYTES: usize = 4096;
// A surviving/escaped descendant can keep stderr open after leader cleanup.
// Preserve final output without allowing that pipe to hold shutdown forever.
const STDERR_DRAIN_BUDGET: std::time::Duration = std::time::Duration::from_secs(2);

pub(crate) struct Workers {
    tasks: JoinSet<Result<WorkerExit, RuntimeError>>,
    reader: Option<AbortHandle>,
    writer: Option<AbortHandle>,
}

impl Workers {
    pub(crate) fn new() -> Self {
        Self {
            tasks: JoinSet::new(),
            reader: None,
            writer: None,
        }
    }

    pub(crate) async fn join_next(
        &mut self,
    ) -> Option<Result<Result<WorkerExit, RuntimeError>, JoinError>> {
        self.tasks.join_next().await
    }

    pub(crate) async fn drain(&mut self) -> Result<(), RuntimeError> {
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
        if let Some(writer) = self.writer.take() {
            writer.abort();
        }
        let result = tokio::time::timeout(STDERR_DRAIN_BUDGET, self.join_after_abort()).await;
        if let Ok(result) = result {
            return result;
        }
        self.tasks.abort_all();
        while self.tasks.join_next().await.is_some() {}
        Err(RuntimeError::StderrDrainTimeout)
    }

    async fn join_after_abort(&mut self) -> Result<(), RuntimeError> {
        let mut failure = None;
        while let Some(joined) = self.tasks.join_next().await {
            let error = match joined {
                Ok(Ok(_exit)) => None,
                // These pipes were intentionally retired above. Any protocol
                // failure selected while live is already retained by Actor;
                // an in-flight write losing its pipe during shutdown is not
                // a new failed process-cleanup acknowledgement.
                Ok(Err(
                    RuntimeError::Read(_) | RuntimeError::Write(_) | RuntimeError::WriteTimeout,
                )) => None,
                Ok(Err(error)) => Some(error),
                Err(cause) if cause.is_cancelled() => None,
                Err(cause) => Some(RuntimeError::Task(crate::TaskFailure::new(cause))),
            };
            // Join every task even when a concrete failure already exists.
            if failure.is_none() {
                failure = error;
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerExit {
    Reader,
    Writer,
    Stderr,
}

pub(crate) fn reader(
    workers: &mut Workers,
    mut stdout: ChildStdout,
    messages: mpsc::Sender<Option<HelperToNode>>,
) {
    workers.reader = Some(workers.tasks.spawn(async move {
        loop {
            let message = wire::read_helper(&mut stdout)
                .await
                .map_err(RuntimeError::Read)?;
            let ended = message.is_none();
            if messages.send(message).await.is_err() || ended {
                return Ok(WorkerExit::Reader);
            }
        }
    }));
}

pub(crate) fn writer(
    workers: &mut Workers,
    mut stdin: ChildStdin,
    mut requests: mpsc::Receiver<NodeToHelper>,
) {
    workers.writer = Some(workers.tasks.spawn(async move {
        while let Some(message) = requests.recv().await {
            tokio::time::timeout(REQUEST_DEADLINE, wire::write_node(&mut stdin, &message))
                .await
                .map_err(|_elapsed| RuntimeError::WriteTimeout)?
                .map_err(RuntimeError::Write)?;
        }
        Ok(WorkerExit::Writer)
    }));
}

pub(crate) fn stderr(workers: &mut Workers, stderr: ChildStderr, logs: SharedLogs) {
    let _stderr = workers.tasks.spawn(read_stderr(stderr, logs));
}

async fn read_stderr(
    mut stderr: impl AsyncRead + Unpin,
    logs: SharedLogs,
) -> Result<WorkerExit, RuntimeError> {
    let mut captured = StderrCapture {
        parser: StderrLines::default(),
        logs,
        eof: false,
    };
    let mut buffer = [0; STDERR_READ_BYTES];
    loop {
        let length = stderr
            .read(&mut buffer)
            .await
            .map_err(RuntimeError::Stderr)?;
        if length == 0 {
            captured.logs.finish_stderr(&mut captured.parser)?;
            captured.eof = true;
            return Ok(WorkerExit::Stderr);
        }
        let bytes = buffer.get(..length).ok_or(RuntimeError::Logs)?;
        captured.logs.stderr(&mut captured.parser, bytes)?;
    }
}

struct StderrCapture {
    parser: StderrLines,
    logs: SharedLogs,
    eof: bool,
}
impl Drop for StderrCapture {
    fn drop(&mut self) {
        if !self.eof {
            // Cancellation/failed reads cannot return a successful complete
            // drain. Preserve the bounded prefix before the join completes.
            // On a poisoned/exhausted store this is best effort: the original
            // worker/timeout failure already prevents lifecycle acknowledgement.
            drop(self.logs.interrupt_stderr(&mut self.parser));
        }
    }
}

#[cfg(test)]
#[path = "workers_tests.rs"]
mod tests;
