//! Stop gates immediately, disposes local devices, then awaits remote cleanup.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::{mpsc, watch};

use crate::cancel::Fence;

const CAPTURE_QUEUE_FRAMES: usize = 100;
const MAX_FRAME_BYTES: usize = 3_200;
const STOP_DEADLINE: Duration = Duration::from_secs(10);

/// This is a local capture frame, not a proposed RPC request.
pub(super) struct Frame(Bytes);

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame")
            .field("bytes", &self.0.len())
            .finish()
    }
}

impl Frame {
    pub(super) fn new(bytes: Bytes) -> Result<Self, CaptureError> {
        if bytes.is_empty()
            || !bytes.len().is_multiple_of(size_of::<i16>())
            || bytes.len() > MAX_FRAME_BYTES
        {
            return Err(CaptureError::Invalid);
        }
        Ok(Self(bytes))
    }

    pub(super) fn pcm(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum CaptureError {
    #[error("capture frame is invalid")]
    Invalid,
    #[error("capture is stopped")]
    Stopped,
    #[error("capture queue overflow stopped the conversation")]
    Overrun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Failure {
    Rejected,
    Disconnected,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Local {
    Capturing,
    Released,
    Uncertain(Failure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Remote {
    Active,
    Stopping,
    Stopped,
    Uncertain(Failure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Status {
    pub(super) local: Local,
    pub(super) remote: Remote,
    pub(super) upload_failure: Option<Failure>,
}

/// Static adapter seam exists before a registered node call; tests never hit a network.
pub(super) trait Adapter: Send {
    fn upload(&mut self, frame: Frame) -> impl Future<Output = Result<(), Failure>> + Send;
    fn stop(&mut self) -> impl Future<Output = Result<(), Failure>> + Send;
}

/// Success means streams were actually stopped/dropped, never merely a queued request.
pub(super) trait Audio: Send {
    fn release(&mut self) -> impl Future<Output = Result<(), Failure>> + Send;
}

#[derive(Debug, Clone)]
pub(super) struct Capture {
    fence: Arc<Fence>,
    tx: mpsc::Sender<Frame>,
}

impl Capture {
    pub(super) fn submit(&self, frame: Frame) -> Result<(), CaptureError> {
        if self.fence.is_closed() {
            return Err(CaptureError::Stopped);
        }
        self.tx.try_send(frame).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => {
                self.fence.close();
                CaptureError::Overrun
            }
            mpsc::error::TrySendError::Closed(_) => CaptureError::Stopped,
        })
    }
}

#[derive(Debug)]
pub(super) struct Handle {
    fence: Arc<Fence>,
    status: watch::Receiver<Status>,
}

impl Handle {
    /// This does not await the actor, the data queue or a remote reply.
    pub(super) fn stop(&self) {
        self.fence.close();
    }
    pub(super) fn status(&self) -> watch::Receiver<Status> {
        self.status.clone()
    }
}

#[derive(Debug)]
pub(super) struct Uploader {
    fence: Arc<Fence>,
    rx: mpsc::Receiver<Frame>,
    status: watch::Sender<Status>,
}

pub(super) fn channel() -> (Capture, Handle, Uploader) {
    with_fence(Arc::new(Fence::default()))
}

pub(super) fn with_fence(fence: Arc<Fence>) -> (Capture, Handle, Uploader) {
    let (tx, rx) = mpsc::channel(CAPTURE_QUEUE_FRAMES);
    let (status, status_rx) = watch::channel(Status {
        local: Local::Capturing,
        remote: Remote::Active,
        upload_failure: None,
    });
    (
        Capture {
            fence: Arc::clone(&fence),
            tx,
        },
        Handle {
            fence: Arc::clone(&fence),
            status: status_rx,
        },
        Uploader { fence, rx, status },
    )
}

impl Uploader {
    pub(super) async fn run(mut self, mut node: impl Adapter, mut audio: impl Audio) {
        let upload_failure = loop {
            match self.upload_one(&mut node).await {
                Ok(Continue::Yes) => {}
                Ok(Continue::No) => break None,
                Err(failure) => break Some(failure),
            }
        };
        self.fence.close();
        self.rx.close();
        while self.rx.try_recv().is_ok() {}
        // Local disposal completes before invoking remote stop; a held remote
        // future cannot retain a microphone or gain another PCM send.
        let local = match tokio::time::timeout(STOP_DEADLINE, audio.release()).await {
            Ok(Ok(())) => Local::Released,
            Ok(Err(error)) => Local::Uncertain(error),
            Err(_) => Local::Uncertain(Failure::TimedOut),
        };
        // Disposal must happen even after release rejects or times out, before
        // any remote wait. The physical capture owner retains its lease until
        // its device thread confirms actual stream destruction.
        drop(audio);
        let _previous = self.status.send_replace(Status {
            local,
            remote: Remote::Stopping,
            upload_failure,
        });
        let remote = match tokio::time::timeout(STOP_DEADLINE, node.stop()).await {
            Ok(Ok(())) => Remote::Stopped,
            Ok(Err(error)) => Remote::Uncertain(error),
            Err(_) => Remote::Uncertain(Failure::TimedOut),
        };
        let _previous = self.status.send_replace(Status {
            local,
            remote,
            upload_failure,
        });
    }

    async fn upload_one(&mut self, node: &mut impl Adapter) -> Result<Continue, Failure> {
        let frame = tokio::select! {
            biased;
            () = self.fence.closed() => return Ok(Continue::No),
            frame = self.rx.recv() => match frame {
                Some(frame) => frame,
                None => return Ok(Continue::No),
            },
        };
        // Stop may have arrived immediately after selecting ready data.
        if self.fence.is_closed() {
            return Ok(Continue::No);
        }
        tokio::select! {
            biased;
            () = self.fence.closed() => Ok(Continue::No),
            result = tokio::time::timeout(STOP_DEADLINE, node.upload(frame)) => match result {
                Ok(Ok(())) => Ok(Continue::Yes),
                Ok(Err(failure)) => Err(failure),
                Err(_) => Err(Failure::TimedOut),
            },
        }
    }
}

#[derive(Debug)]
enum Continue {
    Yes,
    No,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.fence.close();
    }
}

#[cfg(test)]
mod tests;
