//! The websockets a node accepts on `/secure` (axum).

use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use bytes::Bytes;
use futures_util::{Sink, SinkExt as _, Stream, StreamExt as _};
use vessel_proto::NodeClose;

use super::{CLOSE_TIMEOUT, Keepalive, Liveness, NodeCarrier, closed, text_message};

/// A websocket a node accepted, carrying records as binary messages.
///
/// Upgrade with `max_message_size` and `max_frame_size` of
/// [`vessel_secure::MAX_RECORD`], so an oversized message is refused from
/// its header before it is read.
pub struct AxumCarrier {
    socket: WebSocket,
    liveness: Liveness,
}

impl fmt::Debug for AxumCarrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AxumCarrier")
            .field("liveness", &self.liveness)
            .finish_non_exhaustive()
    }
}

impl AxumCarrier {
    /// Carry records over `socket`. With a `keepalive`, ping the client
    /// every `keepalive.ping` and fail after `keepalive.silence` without a
    /// message (its pongs count): what a node's own `/secure` needs, since
    /// no relay pings it.
    #[must_use]
    pub fn new(socket: WebSocket, keepalive: Option<Keepalive>) -> Self {
        Self {
            socket,
            liveness: Liveness::new(keepalive),
        }
    }
}

fn io_error(error: axum::Error) -> io::Error {
    io::Error::other(error)
}

fn ping() -> Message {
    Message::Ping(Bytes::new())
}

impl Stream for AxumCarrier {
    type Item = io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            let message = match Pin::new(&mut this.socket).poll_next(cx) {
                Poll::Ready(Some(Ok(message))) => message,
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(io_error(error)))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {
                    return this
                        .liveness
                        .quiet(Pin::new(&mut this.socket), ping, io_error, cx)
                        .map(|over| Some(Err(over)));
                }
            };
            this.liveness.heard();
            match message {
                Message::Binary(record) => return Poll::Ready(Some(Ok(record))),
                Message::Close(frame) => {
                    return Poll::Ready(closed(frame.map(|frame| frame.code)).map(Err));
                }
                Message::Text(_) => return Poll::Ready(Some(Err(text_message()))),
                Message::Ping(_) | Message::Pong(_) => {}
            }
        }
    }
}

impl Sink<Bytes> for AxumCarrier {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.liveness
            .poll_ready(Pin::new(&mut this.socket), ping, cx)
            .map_err(io_error)
    }

    fn start_send(self: Pin<&mut Self>, record: Bytes) -> io::Result<()> {
        Pin::new(&mut self.get_mut().socket)
            .start_send(Message::Binary(record))
            .map_err(io_error)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let flushed = Pin::new(&mut this.socket).poll_flush(cx).map_err(io_error);
        this.liveness.flushed(flushed)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let closed = Pin::new(&mut this.socket).poll_close(cx).map_err(io_error);
        this.liveness.wrote(closed)
    }
}

impl NodeCarrier for AxumCarrier {
    async fn close_with(mut self, close: NodeClose) {
        let frame = CloseFrame {
            code: close.code(),
            reason: "".into(),
        };
        let closing = async {
            let _sent = self.socket.send(Message::Close(Some(frame))).await;
            let _closed = self.socket.close().await;
            // Until the peer's close or the end: `NodeCarrier::close_with`.
            while let Some(Ok(_unread)) = self.socket.next().await {}
        };
        if tokio::time::timeout(CLOSE_TIMEOUT, closing).await.is_err() {
            tracing::debug!(?close, "closing a connection timed out");
        }
    }
}
