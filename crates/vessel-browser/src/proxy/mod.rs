//! A bounded loopback HTTP/CONNECT proxy for General-profile traffic only.
//!
//! Every connection uses the public resolve/classify/pin primitive; there is
//! no caller-defined resolver, private exception, global proxy or DIRECT path.
//! Node/engine lifetime owns all incoming, upstream and upgraded connections.
//! Preview will need its separate reviewed runtime capability adapter first.

mod closing;
mod forwarding;
pub(crate) use closing::ClosingProxy;
mod request;
#[cfg(test)]
mod tests;

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use crate::preparation::Preparation;
use axum::body::Body;
use axum::http::{Request, Response};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

/// A finite socket/task admission bound, including upgraded tunnels.
const MAX_PROXY_CONNECTIONS: usize = 128;
/// Hyper checks request headers before invoking the forwarding service.
const MAX_PROXY_HEADERS: usize = 64;
/// No per-connection parser can grow past this header-buffer budget.
const MAX_PROXY_HEADER_BYTES: usize = 32 * 1024;
const HEADER_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Lifecycle failures are retained without page-controlled content in messages.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProxyError {
    /// Binding never falls back from loopback to a wildcard/public listener.
    #[error("browser policy proxy could not bind loopback")]
    Bind(#[source] std::io::Error),
    /// A failed listener stops this profile rather than launching without policy.
    #[error("browser policy proxy could not accept a connection")]
    Accept(#[source] std::io::Error),
    /// A failed actor requires the caller to mark the engine unavailable.
    #[error("browser policy proxy task failed")]
    Task(#[source] tokio::task::JoinError),
    /// Shutdown never waits indefinitely for page traffic.
    #[error("browser policy proxy shutdown timed out")]
    Shutdown(#[source] tokio::time::error::Elapsed),
    /// An earlier actor failure can never become a successful second wait.
    #[error("browser policy proxy cleanup could not be confirmed")]
    Unconfirmed,
}

/// Private node-owned proxy; Debug deliberately omits its control endpoint.
pub(crate) struct PublicProxy {
    address: SocketAddr,
    cancel: CancellationToken,
    actor: Option<JoinHandle<Result<(), ProxyError>>>,
}
impl std::fmt::Debug for PublicProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicProxy").finish_non_exhaustive()
    }
}

impl PublicProxy {
    /// Bind an ephemeral loopback endpoint before launching a browser profile.
    ///
    /// # Errors
    /// Any bind/address error refuses; this does not certify engine readiness.
    pub(crate) async fn start() -> Result<Self, ProxyError> {
        let cancel = CancellationToken::new();
        let preparation = Preparation::for_profile(cancel.clone());
        Self::start_scope(cancel, preparation).await
    }

    async fn start_scope(
        cancel: CancellationToken,
        preparation: Preparation,
    ) -> Result<Self, ProxyError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(ProxyError::Bind)?;
        let address = listener.local_addr().map_err(ProxyError::Bind)?;
        let actor = tokio::spawn(listen(listener, cancel.clone(), preparation));
        Ok(Self {
            address,
            cancel,
            actor: Some(actor),
        })
    }

    /// Only the trusted process launcher needs this endpoint; do not serialize/log it.
    #[must_use]
    pub(crate) const fn endpoint(&self) -> SocketAddr {
        self.address
    }

    /// Stop listeners and every stream/tunnel before releasing a profile.
    ///
    /// # Errors
    /// Actor failure or a shutdown deadline is explicit to the engine supervisor.
    pub(crate) async fn close(self) -> Result<(), ProxyError> {
        let mut closing = self.begin_close();
        tokio::time::timeout(SHUTDOWN_TIMEOUT, closing.wait())
            .await
            .map_err(ProxyError::Shutdown)
            .map(|result| result.map(|_stopped| ()))?
    }

    /// Hand off actual cleanup ownership before applying an outer caller deadline.
    /// The process supervisor must retain this until its wait produces a receipt;
    /// an error/timeout never grants permission to release the profile or slot.
    pub(crate) fn begin_close(mut self) -> ClosingProxy {
        self.cancel.cancel();
        ClosingProxy::new(self.actor.take(), self.cancel.clone())
    }
}
impl Drop for PublicProxy {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

#[derive(Clone)]
struct Context {
    cancel: CancellationToken,
    tasks: TaskTracker,
    preparation: Preparation,
    // Kept by upgraded/backend tasks too: an upgrade cannot release admission early.
    permit: Arc<OwnedSemaphorePermit>,
}

async fn listen(
    listener: TcpListener,
    cancel: CancellationToken,
    preparation: Preparation,
) -> Result<(), ProxyError> {
    let tasks = TaskTracker::new();
    let capacity = Arc::new(Semaphore::new(MAX_PROXY_CONNECTIONS));
    let result = loop {
        let accepted = tokio::select! {
            () = cancel.cancelled() => break Ok(()),
            accepted = listener.accept() => accepted,
        };
        let (socket, _) = match accepted {
            Ok(pair) => pair,
            Err(error) => break Err(ProxyError::Accept(error)),
        };
        let Ok(permit) = Arc::clone(&capacity).try_acquire_owned() else {
            // No unbounded wait queue: close excess sockets immediately.
            drop(socket);
            continue;
        };
        let context = Context {
            cancel: cancel.clone(),
            tasks: tasks.clone(),
            preparation: preparation.clone(),
            permit: Arc::new(permit),
        };
        let _task = tasks.spawn(async move {
            let service_context = context.clone();
            let service = service_fn(move |request: Request<Incoming>| {
                let context = service_context.clone();
                async move { Ok::<Response<Body>, Infallible>(forwarding::serve(request, context).await) }
            });
            let mut builder = hyper::server::conn::http1::Builder::new();
            let _configured = builder.timer(TokioTimer::new())
                .header_read_timeout(HEADER_TIMEOUT)
                .max_headers(MAX_PROXY_HEADERS)
                .max_buf_size(MAX_PROXY_HEADER_BYTES);
            tokio::select! {
                () = context.cancel.cancelled() => {},
                _closed = builder.serve_connection(TokioIo::new(socket), service).with_upgrades() => {},
            }
        });
    };
    cancel.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
    preparation.finish().await;
    result
}
