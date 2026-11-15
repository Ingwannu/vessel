//! No raw socket escape: all payload polls retain and recheck the lease.

use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tokio_util::sync::WaitForCancellationFutureOwned;

use super::{PreviewError, preparation::Attempt};

/// The browser's generic connector owns this stream, never its inner socket.
pub(crate) struct PreviewConnection {
    socket: Option<TcpStream>,
    cancelled: Pin<Box<WaitForCancellationFutureOwned>>,
    attempt: Attempt,
}

impl PreviewConnection {
    pub(super) fn new(socket: TcpStream, attempt: Attempt) -> Self {
        let cancelled = Box::pin(attempt.lease.cancelled.clone().cancelled_owned());
        Self {
            socket: Some(socket),
            cancelled,
            attempt,
        }
    }

    fn checked_socket(&mut self, cx: &mut Context<'_>) -> io::Result<&mut TcpStream> {
        let result = if self.cancelled.as_mut().poll(cx).is_ready() {
            Err(PreviewError::Revoked)
        } else {
            self.attempt.lease.validate()
        };
        if let Err(cause) = result {
            // Release the actual socket before any budget/drain owner can go.
            drop(self.socket.take());
            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, cause));
        }
        self.socket
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionAborted, PreviewError::Revoked))
    }
}

impl AsyncRead for PreviewConnection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(self.checked_socket(cx)?).poll_read(cx, buf)
    }
}

impl AsyncWrite for PreviewConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(self.checked_socket(cx)?).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.checked_socket(cx)?).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.checked_socket(cx)?).poll_shutdown(cx)
    }
}

impl Drop for PreviewConnection {
    fn drop(&mut self) {
        drop(self.socket.take());
    }
}

impl std::fmt::Debug for PreviewConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewConnection").finish_non_exhaustive()
    }
}
