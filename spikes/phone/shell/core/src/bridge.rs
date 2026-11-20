//! [`Bridge`]: `vessel-web`'s server, in the app's process.
//!
//! [Decision Log]
//! - Purpose: the web view needs the page and the page needs the bridge;
//!   both are `vessel-web`'s, served as `vessel ui` serves them (ADR
//!   0017), so the phone gets every screen the page has without a second
//!   transport in the page.
//! - Port: one the system picks at each start. A phone's loopback is
//!   shared by every app on it: a fixed port could be taken first by
//!   another app, whose page the web view would then load. The bridge's
//!   origin is fresh at every start anyway (a random host name), and the
//!   page keeps its preferences and drafts in the bridge's files, not in
//!   the origin's storage, so nothing is lost by the port changing.
//! - Address: IPv4 loopback only (the bridge refuses anything else);
//!   nothing listens beyond it.
//! - The address with its one-use bootstrap goes from Rust to the web view
//!   inside the process ([`LaunchUrl`], whose `Debug` hides it): never
//!   logged, never written to a file, never on a command line.
//! - Cost: the bootstrap is spent by the first page that loads it. A web
//!   view made again by Android (the activity recreated while the process
//!   lives) has a new session store and cannot reuse it: the app starts a
//!   new bridge for it ([`Bridge::start`] again), which is cheap.

use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::{CancellationToken, DropGuard};
use vessel_web::{LaunchToken, MintError, ServeError, UiConfig};

use crate::source::PhoneSource;

/// The bridge, serving until it is stopped or dropped.
#[derive(Debug)]
pub struct Bridge {
    url: LaunchUrl,
    address: SocketAddr,
    shutdown: CancellationToken,
    _stop_on_drop: DropGuard,
    served: JoinHandle<Result<(), ServeError>>,
}

/// The address the web view opens: the page, with the launch's one-use
/// bootstrap in its fragment. A credential until the page redeems it.
pub struct LaunchUrl(String);

impl LaunchUrl {
    /// The address, for the web view and nowhere else.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for LaunchUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LaunchUrl(..)")
    }
}

/// Why the bridge did not start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// No loopback port could be bound.
    #[error("no loopback port for the app's bridge")]
    Bind(#[source] io::Error),
    /// No randomness for the launch's secrets.
    #[error(transparent)]
    Mint(#[from] MintError),
}

/// Why the bridge stopped other than by being asked to.
#[derive(Debug, thiserror::Error)]
pub enum StopError {
    /// The server failed.
    #[error(transparent)]
    Serve(#[from] ServeError),
    /// The server's task panicked or was cancelled.
    #[error("the bridge's task ended abnormally")]
    Task(#[from] tokio::task::JoinError),
}

impl Bridge {
    /// Serve the page and the bridge to `source`'s nodes on a loopback
    /// port of the system's choosing. Must be called on a Tokio runtime,
    /// which then runs it.
    ///
    /// # Errors
    /// No port could be bound, or the system gave no randomness.
    pub async fn start(source: PhoneSource) -> Result<Self, StartError> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .map_err(StartError::Bind)?;
        let address = listener.local_addr().map_err(StartError::Bind)?;
        let token = LaunchToken::mint()?;
        let url = LaunchUrl(token.url(address));
        let shutdown = CancellationToken::new();
        let served = tokio::spawn(vessel_web::serve(
            listener,
            UiConfig::new(token),
            source,
            shutdown.child_token(),
        ));
        tracing::info!(%address, "the app's bridge is serving");
        Ok(Self {
            url,
            address,
            _stop_on_drop: shutdown.clone().drop_guard(),
            shutdown,
            served,
        })
    }

    /// What the web view opens.
    #[must_use]
    pub const fn url(&self) -> &LaunchUrl {
        &self.url
    }

    /// Where it listens.
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    /// Stop serving and wait for the server (its drafts' last saves
    /// included).
    ///
    /// # Errors
    /// The server had failed, or its task ended abnormally.
    pub async fn stop(self) -> Result<(), StopError> {
        self.shutdown.cancel();
        self.served.await??;
        Ok(())
    }
}
