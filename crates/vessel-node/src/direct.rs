//! The direct route's accept loop (`/health`, `/rpc`, `/secure` on the
//! node's TCP address): every socket counted from `accept`, HTTP/1 with a
//! deadline on each request's headers, and a stop that waits a while and
//! then drops what is left.
//!
//! [Decision Log]
//! - Problem: under `axum::serve` nothing bounded the direct route's
//!   sockets or how long one took to send its headers (axum sets no
//!   timer, so hyper waits for good). The `/secure` budgets
//!   (`secure::MAX_UNADMITTED`) apply only after the websocket upgrade.
//!   A node listening beyond loopback (`--public`) could be sent any
//!   number of half requests until it ran out of descriptors, which its
//!   sessions, store and files share, and a stop waited on every one.
//! - Choice: the relay's pattern (`vessel-relay`'s `server::accept`) and
//!   the model proxy's (`models::listen`): our own loop over `hyper`'s
//!   HTTP/1 connection with a timer and [`Limits::header_read`]. Before
//!   any byte is read a socket takes a place among
//!   [`Limits::connections`] and, unless it comes from loopback or a
//!   trusted proxy, one among [`Limits::per_address`] for its address
//!   (an IPv4 address, or an IPv6 `/64`, `/56` and `/48` scaled as
//!   `vessel_relay::address` scales them); without one it is answered
//!   `503` and closed. The place travels with the socket ([`Held`])
//!   through a websocket upgrade, until it closes.
//! - Loopback is not counted per address: it is this machine (its own
//!   tools and the web UI's bridge), and the cap in all still bounds it.
//!   A trusted proxy's sockets are not either: every client of the proxy
//!   would be one address; the proxy limits per client, as the README
//!   asks of it.
//! - Alternatives: a timeout around each connection's whole life (cuts
//!   long-lived websockets); a per-address rate as the relay keeps (a node
//!   serves its own devices, not the public; the cap and the deadline
//!   bound what one host holds).
//! - Cost: HTTP/2 is not served (every client speaks HTTP/1 websockets).

use std::io;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll};
use std::time::Duration;

use axum::Router;
use axum::extract::ConnectInfo;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt as _;
use vessel_relay::address::{Block, BlockCounts, ClientAddress, blocks};

/// What a socket past a limit is told before it is closed.
const REFUSED: &[u8] =
    b"HTTP/1.1 503 Service Unavailable\r\nconnection: close\r\ncontent-length: 0\r\n\r\n";

/// The direct route's bounds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// Sockets open at once, of every kind, upgraded ones included.
    pub(crate) connections: usize,
    /// Of those, how many one address holds (scaled for wider IPv6
    /// blocks). Four times it stays under half of `connections`, so one
    /// `/48` never holds half the route.
    pub(crate) per_address: usize,
    /// How long a connection may take to send a request's headers; also
    /// how long an idle keep-alive connection waits for its next one.
    pub(crate) header_read: Duration,
    /// How long the connections may finish when the node stops.
    pub(crate) stop_grace: Duration,
}

impl Limits {
    /// The node's own: a person's devices and the web UI's bridge hold a
    /// few each; the per-address share leaves room behind one NAT.
    pub(crate) const NODE: Self = Self {
        connections: 512,
        per_address: 32,
        header_read: Duration::from_secs(10),
        stop_grace: Duration::from_secs(5),
    };
}

/// Serve `app` on `listener` until `stop`, then stop as the module says.
pub(crate) async fn run(
    listener: TcpListener,
    app: Router,
    clients: ClientAddress,
    limits: Limits,
    stop: CancellationToken,
) {
    let places = Places {
        all: Arc::new(Semaphore::new(limits.connections)),
        per_address: Arc::default(),
        limit: limits.per_address,
    };
    let mut connections = JoinSet::new();
    loop {
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            Some(_ended) = connections.join_next(), if !connections.is_empty() => continue,
            () = stop.cancelled() => break,
        };
        let (stream, peer) = match accepted {
            Ok(accepted) => accepted,
            Err(error) if is_connection_error(&error) => continue,
            Err(error) => {
                // Out of descriptors, most likely: they free up as
                // connections end, so wait a moment rather than spin.
                tracing::warn!(%error, "accepting a direct connection failed");
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_millis(100)) => continue,
                    () = stop.cancelled() => break,
                }
            }
        };
        let counted = !peer.ip().is_loopback() && !clients.is_trusted_proxy(peer.ip());
        let Some(place) = places.take(peer.ip(), counted) else {
            // Best effort and never waiting: a client that reads learns
            // why, and nothing more is spent on it.
            let _written = stream.try_write(REFUSED);
            tracing::debug!(peer = %peer.ip(), "refused a direct connection past a limit");
            continue;
        };
        let _nodelay = stream.set_nodelay(true);
        let io = Held {
            stream,
            _place: place,
        };
        let service = TowerToHyperService::new(app.clone().map_request(
            move |mut request: hyper::Request<Incoming>| {
                let _previous = request.extensions_mut().insert(ConnectInfo(peer));
                request
            },
        ));
        drop(connections.spawn(serve_connection(
            io,
            service,
            limits.header_read,
            stop.clone(),
        )));
    }
    let left = tokio::time::timeout(limits.stop_grace, async {
        while connections.join_next().await.is_some() {}
    })
    .await;
    if left.is_err() {
        tracing::info!(
            connections = connections.len(),
            "stopping: dropped the direct connections still open after the grace"
        );
    }
    connections.shutdown().await;
}

/// Errors of one connection, not of the listener.
fn is_connection_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

/// One connection: HTTP/1 with the header deadline and upgrades, until it
/// ends, or until `stop` asks it to finish what it is doing.
async fn serve_connection<S>(io: Held, service: S, header_read: Duration, stop: CancellationToken)
where
    S: hyper::service::Service<hyper::Request<Incoming>, Response = axum::response::Response>
        + Send,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let mut builder = http1::Builder::new();
    let _configured = builder
        .timer(TokioTimer::new())
        .header_read_timeout(header_read);
    let connection = builder
        .serve_connection(TokioIo::new(io), service)
        .with_upgrades();
    tokio::pin!(connection);
    let served = tokio::select! {
        served = connection.as_mut() => served,
        () = stop.cancelled() => {
            connection.as_mut().graceful_shutdown();
            connection.await
        }
    };
    if let Err(error) = served {
        tracing::debug!(%error, "a direct connection ended with an error");
    }
}

/// The places sockets take.
struct Places {
    all: Arc<Semaphore>,
    per_address: Arc<Mutex<BlockCounts>>,
    limit: usize,
}

impl Places {
    /// A place for a socket from `peer`, counted per address if `counted`;
    /// `None` past a limit.
    fn take(&self, peer: IpAddr, counted: bool) -> Option<Place> {
        let all = Arc::clone(&self.all).try_acquire_owned().ok()?;
        let address = if counted {
            let blocks = blocks(peer);
            let mut counts = self
                .per_address
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if !counts.fits(&blocks, self.limit) {
                return None;
            }
            counts.add(&blocks);
            Some((Arc::clone(&self.per_address), blocks))
        } else {
            None
        };
        Some(Place { _all: all, address })
    }
}

/// A socket's places, given back when it closes.
struct Place {
    _all: OwnedSemaphorePermit,
    address: Option<(Arc<Mutex<BlockCounts>>, Vec<Block>)>,
}

impl Drop for Place {
    fn drop(&mut self) {
        if let Some((counts, blocks)) = &self.address {
            counts
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(blocks);
        }
    }
}

/// A socket holding its place: it travels with the socket through hyper and
/// a websocket upgrade, so the place is held exactly as long as the socket.
struct Held {
    stream: TcpStream,
    _place: Place,
}

impl AsyncRead for Held {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for Held {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests;
