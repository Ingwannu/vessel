//! A single opened local transport, before any credential or protocol output.
//!
//! An owning launcher must qualify the actual peer before authentication. Keeping
//! open and credential output separate prevents a replacement endpoint from
//! receiving a token while its process identity is still unproven. This value
//! never reconnects, clones a handle, signals a PID or grants process ownership.

use std::io;
use std::num::NonZeroU32;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[cfg(unix)]
type Stream = tokio::net::UnixStream;
#[cfg(windows)]
type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

/// A retained local handle; namespace reopening always requires a new value.
pub struct LocalTransport {
    stream: Stream,
}

impl std::fmt::Debug for LocalTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LocalTransport(<retained handle>)")
    }
}

impl LocalTransport {
    /// Open without sending any bytes, including credentials or a Hello frame.
    ///
    /// # Errors
    /// The platform's local connection error, without retaining the endpoint.
    pub async fn open(endpoint: &Path) -> io::Result<Self> {
        Ok(Self {
            stream: open(endpoint).await?,
        })
    }

    /// Kernel-reported process of this original peer, not signal authority.
    ///
    /// Missing platform support returns None. A launcher must refuse ownership
    /// unless it has a separate qualified proof on this same retained handle.
    ///
    /// # Errors
    /// The kernel query fails or returns an invalid process representation.
    #[cfg(unix)]
    pub fn peer_process(&self) -> io::Result<Option<NonZeroU32>> {
        self.stream
            .peer_cred()?
            .pid()
            .map(|pid| {
                let raw = u32::try_from(pid).map_err(io::Error::other)?;
                NonZeroU32::new(raw).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "zero local peer process")
                })
            })
            .transpose()
    }

    /// Safe named-pipe APIs do not expose a qualified server PID here.
    ///
    /// # Errors
    /// This implementation explicitly refuses and performs no unsafe fallback.
    #[cfg(windows)]
    pub fn peer_process(&self) -> io::Result<Option<NonZeroU32>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "local peer process is unverified",
        ))
    }
}

impl AsyncRead for LocalTransport {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for LocalTransport {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

#[cfg(unix)]
async fn open(endpoint: &Path) -> io::Result<Stream> {
    Stream::connect(endpoint).await
}

#[cfg(windows)]
async fn open(endpoint: &Path) -> io::Result<Stream> {
    use std::time::Duration;
    use tokio::net::windows::named_pipe::ClientOptions;
    const ERROR_PIPE_BUSY: i32 = 231;
    const ATTEMPTS: u32 = 50;
    const PAUSE: Duration = Duration::from_millis(20);
    for _ in 0..ATTEMPTS {
        match ClientOptions::new().open(endpoint) {
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                tokio::time::sleep(PAUSE).await
            }
            opened => return opened,
        }
    }
    ClientOptions::new().open(endpoint)
}
