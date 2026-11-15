//! One run's pipe workers: final proof requires real EOF and completed task drops.
use super::TermLog;
use crate::lifecycle::drain_deadline;
use bytes::{Bytes, BytesMut};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _},
    sync::mpsc,
    task::JoinHandle,
};

const CHUNK: usize = 16 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Drained {
    Complete,
    Incomplete,
}
#[derive(Debug)]
pub(super) struct Pumps(JoinHandle<Drained>);
impl Drop for Pumps {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl Pumps {
    pub(super) fn start(
        stdout: impl AsyncRead + Unpin + Send + 'static,
        stderr: impl AsyncRead + Unpin + Send + 'static,
        sender: mpsc::Sender<Bytes>,
    ) -> Self {
        Self(tokio::spawn(async move {
            let (out, err) = tokio::join!(pump(stdout, sender.clone()), pump(stderr, sender));
            if out == Drained::Complete && err == Drained::Complete {
                Drained::Complete
            } else {
                Drained::Incomplete
            }
        }))
    }
    /// A deadline stops publication, then joins cancelled workers. It cannot
    /// turn incomplete output into a successful cleanup report.
    pub(super) async fn finish(
        mut self,
        log: &mut TermLog,
        output: &mut mpsc::Receiver<Bytes>,
        open: bool,
    ) -> Drained {
        let drained = if open {
            drain(log, output).await
        } else {
            Drained::Complete
        };
        if drained == Drained::Incomplete {
            self.0.abort();
        }
        match (&mut self.0).await {
            Ok(Drained::Complete) if drained == Drained::Complete => Drained::Complete,
            Ok(Drained::Complete | Drained::Incomplete) | Err(_) => Drained::Incomplete,
        }
    }
}
async fn drain(log: &mut TermLog, output: &mut mpsc::Receiver<Bytes>) -> Drained {
    let deadline = drain_deadline();
    loop {
        tokio::select! {
            chunk = output.recv() => match chunk {
                Some(bytes) => log.output(bytes),
                None => return Drained::Complete,
            },
            () = tokio::time::sleep_until(deadline) => return Drained::Incomplete,
        }
    }
}
async fn pump(mut stream: impl AsyncRead + Unpin, sender: mpsc::Sender<Bytes>) -> Drained {
    let mut buffer = BytesMut::with_capacity(CHUNK);
    loop {
        if buffer.capacity() < CHUNK {
            buffer.reserve(CHUNK);
        }
        match stream.read_buf(&mut buffer).await {
            Ok(0) => return Drained::Complete,
            Err(_) => return Drained::Incomplete,
            Ok(_) => {
                if sender.send(buffer.split().freeze()).await.is_err() {
                    return Drained::Incomplete;
                }
            }
        }
    }
}
