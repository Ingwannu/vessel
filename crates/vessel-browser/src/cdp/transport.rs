//! Private debugger I/O. Raw bytes here are framing buffers, never domain payloads.
//!
//! Pipe reads retain their buffer in the owner across cancelled `next()` futures.
//! No EOF, malformed stdout or failed pipe can select a WebSocket fallback.
use super::client::{CdpError, WRITE_TIMEOUT};
use super::wire::MAX_MESSAGE_BYTES;
use futures_util::{SinkExt as _, StreamExt as _};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use vessel_proto::browser::BrowserEffect;

/// A fixed transport set, statically dispatched; this is not a plugin/RPC extension.
pub(super) trait Transport: Send + 'static {
    fn send(&mut self, text: String) -> impl Future<Output = Result<(), CdpError>> + Send;
    fn next(&mut self) -> impl Future<Output = Result<Option<String>, CdpError>> + Send;
}
pub(super) struct WebSocket(pub(super) WebSocketStream<MaybeTlsStream<TcpStream>>);
impl Transport for WebSocket {
    async fn send(&mut self, text: String) -> Result<(), CdpError> {
        self.0
            .send(Message::Text(text.into()))
            .await
            .map_err(CdpError::Socket)
    }
    async fn next(&mut self) -> Result<Option<String>, CdpError> {
        loop {
            match self.0.next().await {
                Some(Ok(Message::Text(text))) => return Ok(Some(text.to_string())),
                Some(Ok(Message::Ping(_))) => {
                    tokio::time::timeout(WRITE_TIMEOUT, self.0.flush())
                        .await
                        .map_err(|source| CdpError::TimedOut {
                            effect: BrowserEffect::NotDelivered,
                            source,
                        })?
                        .map_err(CdpError::Socket)?;
                }
                Some(Ok(Message::Pong(_))) => {}
                Some(Ok(Message::Close(_))) | None => return Ok(None),
                Some(Ok(Message::Binary(_) | Message::Frame(_))) => return Err(CdpError::Protocol),
                Some(Err(error)) => return Err(CdpError::Socket(error)),
            }
        }
    }
}

/// At most one read quantum is inspected before applying the message bound.
const PIPE_READ_BYTES: usize = 8192;
#[cfg(test)]
pub(in crate::cdp) const PIPE_TEST_CAPACITY: usize = PIPE_READ_BYTES;
/// Handles are owned by the supervised child launcher, not a URL or public socket.
pub(in crate::cdp) struct Pipe<R, W> {
    read: R,
    write: W,
    buffer: Vec<u8>,
    scanned: usize,
}
impl<R, W> Pipe<R, W> {
    pub(in crate::cdp) fn new(read: R, write: W) -> Self {
        Self {
            read,
            write,
            buffer: Vec::new(),
            scanned: 0,
        }
    }
}
impl<R, W> Transport for Pipe<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    async fn send(&mut self, text: String) -> Result<(), CdpError> {
        if text.len() > MAX_MESSAGE_BYTES || text.contains('\0') {
            return Err(CdpError::Size);
        }
        self.write
            .write_all(text.as_bytes())
            .await
            .map_err(CdpError::Pipe)?;
        self.write.write_all(&[0]).await.map_err(CdpError::Pipe)?;
        self.write.flush().await.map_err(CdpError::Pipe)
    }
    async fn next(&mut self) -> Result<Option<String>, CdpError> {
        let mut chunk = [0_u8; PIPE_READ_BYTES];
        loop {
            let suffix = self.buffer.get(self.scanned..).ok_or(CdpError::Protocol)?;
            if let Some(relative) = suffix.iter().position(|byte| *byte == 0) {
                let end = self.scanned.checked_add(relative).ok_or(CdpError::Size)?;
                if end > MAX_MESSAGE_BYTES {
                    return Err(CdpError::Size);
                }
                let mut bytes = self.buffer.drain(..=end).collect::<Vec<_>>();
                let _delimiter = bytes.pop();
                self.scanned = 0;
                // Utf8Error retains no original text/bytes in the error chain.
                let text = std::str::from_utf8(&bytes)
                    .map_err(CdpError::Utf8)?
                    .to_owned();
                return Ok(Some(text));
            }
            self.scanned = self.buffer.len();
            if self.buffer.len() > MAX_MESSAGE_BYTES {
                return Err(CdpError::Size);
            }
            let count = self.read.read(&mut chunk).await.map_err(CdpError::Pipe)?;
            if count == 0 {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(CdpError::Protocol)
                };
            }
            let read = chunk.get(..count).ok_or(CdpError::Protocol)?;
            // One read may contain several valid messages. The next iteration
            // first finds a delimiter, then checks its exact message length.
            self.buffer.extend_from_slice(read);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    const FIXTURE_TIMEOUT: Duration = Duration::from_secs(20);
    #[tokio::test]
    async fn cancelled_partial_read_preserves_bytes_and_multi_message_boundaries() {
        tokio::time::timeout(FIXTURE_TIMEOUT,async {
            let (node,mut peer)=tokio::io::duplex(PIPE_READ_BYTES);
            let (read,write)=tokio::io::split(node);
            let mut pipe=Pipe::new(read,write);
            peer.write_all(b"{\"id\":").await.unwrap();
            tokio::select!{biased;_message=pipe.next()=>panic!("incomplete frame completed"),()=tokio::task::yield_now()=>{}}
            peer.write_all(b"1}\0{\"id\":2}\0").await.unwrap();
            assert_eq!(pipe.next().await.unwrap().unwrap(),"{\"id\":1}");
            assert_eq!(pipe.next().await.unwrap().unwrap(),"{\"id\":2}");
            pipe.send("{\"id\":3}".to_owned()).await.unwrap();
            let mut answer=[0;9];let _read = peer.read_exact(&mut answer).await.unwrap();
            assert_eq!(&answer,b"{\"id\":3}\0");
            peer.write_all(b"unterminated").await.unwrap();peer.shutdown().await.unwrap();
            assert!(matches!(pipe.next().await,Err(CdpError::Protocol)));
        }).await.unwrap();
    }
    #[tokio::test]
    async fn oversized_or_invalid_utf8_stdout_cannot_become_a_control_frame() {
        tokio::time::timeout(FIXTURE_TIMEOUT, async {
            let (node, mut peer) = tokio::io::duplex(PIPE_READ_BYTES);
            let (read, write) = tokio::io::split(node);
            let mut pipe = Pipe::new(read, write);
            peer.write_all(&[255, 0]).await.unwrap();
            assert!(matches!(pipe.next().await, Err(CdpError::Utf8(_))));
            let sender = tokio::spawn(async move {
                let bytes = vec![b'x'; MAX_MESSAGE_BYTES.saturating_add(1)];
                let _closed = peer.write_all(&bytes).await;
            });
            assert!(matches!(pipe.next().await, Err(CdpError::Size)));
            drop(pipe);
            sender.await.unwrap();
        })
        .await
        .unwrap();
    }
}
