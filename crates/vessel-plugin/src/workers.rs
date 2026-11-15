//! Each pipe has one cancellation owner; selecting another actor event cannot
//! discard a half-read header or a half-written frame.

use tokio::{
    io::AsyncReadExt,
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::mpsc,
    task::JoinSet,
};
use vessel_proto::plugins::{HelperToNode, NodeToHelper};

use crate::{
    log_tail::StderrLines, runtime_error::RuntimeError, runtime_types::REQUEST_DEADLINE,
    shared_logs::SharedLogs, wire,
};

const STDERR_READ_BYTES: usize = 4096;

pub(crate) type Workers = JoinSet<Result<WorkerExit, RuntimeError>>;

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
    let _reader = workers.spawn(async move {
        loop {
            let message = wire::read_helper(&mut stdout)
                .await
                .map_err(RuntimeError::Read)?;
            let ended = message.is_none();
            if messages.send(message).await.is_err() || ended {
                return Ok(WorkerExit::Reader);
            }
        }
    });
}

pub(crate) fn writer(
    workers: &mut Workers,
    mut stdin: ChildStdin,
    mut requests: mpsc::Receiver<NodeToHelper>,
) {
    let _writer = workers.spawn(async move {
        while let Some(message) = requests.recv().await {
            tokio::time::timeout(REQUEST_DEADLINE, wire::write_node(&mut stdin, &message))
                .await
                .map_err(|_elapsed| RuntimeError::WriteTimeout)?
                .map_err(RuntimeError::Write)?;
        }
        Ok(WorkerExit::Writer)
    });
}

pub(crate) fn stderr(workers: &mut Workers, mut stderr: ChildStderr, logs: SharedLogs) {
    let _stderr = workers.spawn(async move {
        let mut buffer = [0; STDERR_READ_BYTES];
        let mut parser = StderrLines::default();
        loop {
            let length = stderr
                .read(&mut buffer)
                .await
                .map_err(RuntimeError::Stderr)?;
            if length == 0 {
                logs.finish_stderr(&mut parser)?;
                return Ok(WorkerExit::Stderr);
            }
            let bytes = buffer.get(..length).ok_or(RuntimeError::Logs)?;
            logs.stderr(&mut parser, bytes)?;
        }
    });
}
