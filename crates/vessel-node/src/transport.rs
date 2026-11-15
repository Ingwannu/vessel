//! axum's websocket adapted to `vessel-rpc`'s frame transport: one text
//! message per frame. axum answers the client's pings itself; a binary
//! message is an error, since protocol 2 is text frames only.
//!
//! [Decision Log]
//! - Purpose: a client that vanished without closing (a laptop that slept,
//!   a network that dropped, a process that no longer reads) must not keep
//!   its connection, and with it its subscriptions and the sessions they
//!   follow, for good. `/secure` had a keepalive from the start; `/rpc`
//!   had none, so the node only noticed such a client when it next wrote
//!   to it, which for an idle session's subscription is never (found
//!   staging parking, 2026-11-14).
//! - Choice: the node pings its `/rpc` connections as it pings `/secure`
//!   ones, with the same numbers ([`Keepalive`]) and the same code
//!   ([`Liveness`]), and drops one silent past the limit. Every client
//!   answers pings without help: tungstenite while it reads, browsers on
//!   their own.
//! - Cost: a ping every 10 s per idle connection.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::extract::ws::{Message, WebSocket};
use bytes::Bytes;
use futures_util::{Sink, Stream};
use vessel_relay::{Keepalive, Liveness};

/// Why the websocket transport failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TransportError {
    /// The websocket itself failed.
    #[error("websocket failed")]
    Websocket(#[from] axum::Error),
    /// The client sent a binary message.
    #[error("the client sent a binary message; protocol 2 uses text frames only")]
    BinaryFrame,
    /// The client went silent past the keepalive's limit, or a ping to it
    /// failed.
    #[error("the client went silent")]
    Silent(#[source] io::Error),
}

/// A client's websocket carrying protocol frames.
#[derive(Debug)]
pub(crate) struct WsTransport {
    socket: WebSocket,
    liveness: Liveness,
}

impl WsTransport {
    /// Carry frames over `socket`, pinging the client and dropping it
    /// after `keepalive`'s silence; `None` for neither.
    pub(crate) fn new(socket: WebSocket, keepalive: Option<Keepalive>) -> Self {
        Self {
            socket,
            liveness: Liveness::new(keepalive),
        }
    }
}

fn ping() -> Message {
    Message::Ping(Bytes::new())
}

impl Stream for WsTransport {
    type Item = Result<String, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            let message = match Pin::new(&mut this.socket).poll_next(cx) {
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error.into()))),
                Poll::Ready(Some(Ok(message))) => message,
                Poll::Pending => {
                    return this
                        .liveness
                        .quiet(Pin::new(&mut this.socket), ping, io::Error::other, cx)
                        .map(|over| Some(Err(TransportError::Silent(over))));
                }
            };
            this.liveness.heard();
            match message {
                Message::Text(text) => return Poll::Ready(Some(Ok(text.as_str().to_owned()))),
                Message::Binary(_) => return Poll::Ready(Some(Err(TransportError::BinaryFrame))),
                Message::Close(_) => return Poll::Ready(None),
                Message::Ping(_) | Message::Pong(_) => {}
            }
        }
    }
}

impl Sink<String> for WsTransport {
    type Error = TransportError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        this.liveness
            .poll_ready(Pin::new(&mut this.socket), ping, cx)
            .map_err(Into::into)
    }

    fn start_send(self: Pin<&mut Self>, frame: String) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().socket)
            .start_send(Message::Text(frame.into()))
            .map_err(Into::into)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let flushed = Pin::new(&mut this.socket)
            .poll_flush(cx)
            .map_err(Into::into);
        this.liveness.flushed(flushed)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let closed = Pin::new(&mut this.socket)
            .poll_close(cx)
            .map_err(Into::into);
        this.liveness.wrote(closed)
    }
}
