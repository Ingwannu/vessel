//! The member pull loop: fetch the home's bundle, apply and report.
//! Joining and leaving live in `joining`, shared by the offline CLI and RPC.
//!
//! [Decision Log]
//! - Membership is a pairing (ADR 0012): the home makes an invitation
//!   with the `sync` scope (`vessel fleet invite`), and `join` redeems it
//!   with `vessel_client::secure::pair_over` exactly as a device does. The
//!   credential and the home's routes are stored in the data directory
//!   (`fleet/member.credential`, `fleet/member.json`, owner-only), where
//!   the node finds them within a few seconds, running or not.
//! - The link is a single-use secret: `join` takes the decoded offer, and
//!   the CLI reads the link from standard input, never from the command
//!   line (`vessel fleet join`, as `vessel nodes add --pair`).
//! - When: on start (and whenever the connection comes back), when
//!   `fleet.watch` answers with another generation, and every 10 minutes
//!   regardless (`FleetConfig::pull_every`), which also notices local
//!   changes such as a restored file. Each pull fetches the manifest, the
//!   blobs the member lacks (chunk by chunk, hash-checked), applies into
//!   every profile's home, and reports the generation and the conflicted
//!   bundle paths.
//! - Failures: a connection that cannot be made or is lost is retried with
//!   the client's backoff (0.5 s to 30 s, jittered); a failed pull waits
//!   the same before the next, so a broken disk does not spin. A connection
//!   counts as working only once the home answered a pull or a watch: one
//!   that ends before that waits the backoff, so a home that accepts and
//!   drops at once is not granted again in a tight loop. Each is in
//!   `state.json` for `vessel fleet status`.
//! - The mutual grant (protocol 6, Track X): after connecting, the loop
//!   gives the home a credential on this node with the scopes the owner
//!   chose at join (`links`), and shares its connection for calls this
//!   node relays through the home.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use vessel_client::secure::{SecureConnector, TlsError, WsDialer};
use vessel_client::{
    Backoff, CallError, Connector, CredentialError, CredentialMismatch, DeviceCredential,
    NodeClient, Pinned, SecureTarget,
};
use vessel_fleet::BlobStore;
use vessel_proto::{ClientInfo, Generation, Manifest, NodeId, NodePublicKey, RouteUrl, Scope};

use vessel_client::FetchError;

use super::chunks;
use super::files::{self, FileError, FleetFiles};
use super::links::Links;
use super::local::Local;

/// The home this node is a member of (`fleet/member.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Membership {
    /// The home node.
    pub(crate) home: NodeId,
    /// Its key.
    pub(crate) key: NodePublicKey,
    /// Its relays.
    #[serde(default)]
    pub(crate) relays: Vec<RouteUrl>,
    /// Its direct addresses.
    #[serde(default)]
    pub(crate) direct: Vec<RouteUrl>,
    /// When this node joined.
    pub(crate) joined_at: DateTime<Utc>,
    /// What the home may do on this node (protocol 6, `vessel fleet join
    /// --grant`): its credential here is made with these. Empty grants
    /// nothing, as for a membership from before grants.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) grant: Vec<Scope>,
}

impl Membership {
    fn target(&self) -> SecureTarget {
        SecureTarget {
            key: self.key,
            relays: self.relays.clone(),
            direct: self.direct.clone(),
        }
    }
}

/// Why the node cannot pull with its membership.
#[derive(Debug, thiserror::Error)]
pub(crate) enum MembershipError {
    #[error(transparent)]
    File(#[from] FileError),
    #[error(transparent)]
    Credential(#[from] CredentialError),
    #[error("the credential is not for the home in member.json: {0}")]
    Mismatch(#[from] CredentialMismatch),
    #[error("TLS for the home's relays: {0}")]
    Tls(#[source] TlsError),
}

/// The membership in `files`, if any. Blocks.
pub(crate) fn membership(files: &FleetFiles) -> Result<Option<Membership>, FileError> {
    files::read_json(&files.membership())
}

/// The connector to the home of `membership`. Blocks (reads the
/// credential).
pub(crate) fn connector(
    files: &FleetFiles,
    membership: &Membership,
) -> Result<Pinned<SecureConnector<WsDialer>>, MembershipError> {
    let credential = DeviceCredential::read(&files.credential())?;
    let target = membership.target();
    let dialer = WsDialer::for_routes(target.routes()).map_err(MembershipError::Tls)?;
    let connector = SecureConnector::new(dialer, target, membership.home, credential)?;
    Ok(Pinned::new(connector, Some(membership.home)))
}

/// How the pull loop runs.
#[derive(Debug, Clone)]
pub(crate) struct Pacing {
    /// Pull at least this often.
    pub(crate) pull_every: Duration,
    /// Waits after failures.
    pub(crate) backoff: Backoff,
    /// One coalescing manual request, including during a retry wait.
    pub(crate) manual: Arc<tokio::sync::Notify>,
}

fn client_info() -> ClientInfo {
    ClientInfo {
        name: "vesseld-fleet".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// Why a pull failed.
#[derive(Debug, thiserror::Error)]
enum PullError {
    #[error("the home: {0}")]
    Call(#[from] CallError),
    #[error("a blob: {0}")]
    Blob(#[source] std::io::Error),
    #[error("a blob arrived longer than the home said ({0} bytes)")]
    Oversized(u64),
    #[error(transparent)]
    File(#[from] FileError),
    #[error(transparent)]
    Apply(#[from] super::local::LocalError),
    #[error("a blocking task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

/// What a member grants its home unless its owner chooses: enough for the
/// home to pass on other nodes' calls to read and drive sessions here,
/// never `admin`.
pub const DEFAULT_GRANT: [Scope; 2] = [Scope::Read, Scope::Operate];

/// Pull from the home of `membership` through `connector` until `stop`,
/// sharing the connection with `links` while it lasts.
pub(crate) async fn run<C: Connector>(
    connector: C,
    membership: Membership,
    local: Arc<Local>,
    links: Arc<Links>,
    pacing: Pacing,
    stop: CancellationToken,
) {
    let mut failures = 0_u32;
    let mut applied = local.state.get().generation;
    while !stop.is_cancelled() {
        let client = tokio::select! {
            connected = connector.connect(client_info()) => connected,
            () = stop.cancelled() => return,
        };
        let client = match client {
            Ok(client) => client,
            Err(error) => {
                let message = format!("cannot reach the home: {error}");
                tracing::warn!("{message}");
                note_error(&local, message).await;
                failures = failures.saturating_add(1);
                match pause(pacing.backoff.delay(failures), &stop, &pacing.manual).await {
                    Wake::Stopped => return,
                    Wake::Elapsed | Wake::Manual => {}
                }
                continue;
            }
        };
        tracing::info!(home = %client.node().id, "connected to the fleet's home");
        links.connected_home(client.clone());
        let granted = links.grant(&client, &membership, &stop).await;
        // A cancelled grant is shutdown, not admission to the first pull.
        // Keep the cached connection's lifetime aligned with this loop even
        // when cancellation arrives during the state write below.
        if stop.is_cancelled() {
            links.lost_home();
            return;
        }
        note_grant(&local, granted.err().map(|error| error.message)).await;
        if stop.is_cancelled() {
            links.lost_home();
            return;
        }
        // A member called back through its home's relays asks for a newer
        // ticket while connected (`fleet.callback`); the connection's end
        // stops that. The callback running counts as the home having
        // answered: the connection was in use.
        let contact = tokio::select! {
            contact = session(&client, &local, &pacing, &mut applied, &stop) => contact,
            () = links.keep_callback(&client, &membership) => Contact::Answered,
        };
        links.lost_home();
        // A connection the home answered on was a working one: reconnect at
        // once. One that ended before the home answered anything counts as
        // a failure, so a home that accepts and drops at once is not dialled
        // (and granted) in a tight loop.
        match contact {
            Contact::Answered => failures = 0,
            Contact::None => {
                failures = failures.saturating_add(1);
                match pause(pacing.backoff.delay(failures), &stop, &pacing.manual).await {
                    Wake::Stopped => return,
                    Wake::Elapsed | Wake::Manual => {}
                }
            }
        }
    }
}

/// Whether the home answered during one connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contact {
    /// A pull or a watch got an answer.
    Answered,
    /// The connection ended before any answer.
    None,
}

/// One connection's worth: pull, then wait for news or the timer.
async fn session(
    client: &NodeClient,
    local: &Arc<Local>,
    pacing: &Pacing,
    applied: &mut Option<Generation>,
    stop: &CancellationToken,
) -> Contact {
    let mut contact = Contact::None;
    let mut failures = 0_u32;
    let mut due = true;
    let mut timer = tokio::time::interval(pacing.pull_every);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let _first = timer.tick().await;
    loop {
        // Watch/timer and cancellation may be ready together. A select arm
        // is not permission to begin new disk work after shutdown; an apply
        // already admitted still finishes normally.
        if stop.is_cancelled() {
            return contact;
        }
        if due {
            match pull(client, local).await {
                Ok(generation) => {
                    *applied = Some(generation);
                    failures = 0;
                    contact = Contact::Answered;
                }
                Err(PullError::Call(error)) if is_lost(&error) => {
                    tracing::info!(%error, "lost the fleet's home; reconnecting");
                    return contact;
                }
                Err(error) => {
                    let message = format!("pulling from the home: {error}");
                    tracing::warn!("{message}");
                    note_error(local, message).await;
                    failures = failures.saturating_add(1);
                    match pause(pacing.backoff.delay(failures), stop, &pacing.manual).await {
                        Wake::Stopped => return contact,
                        Wake::Elapsed | Wake::Manual => {}
                    }
                    if stop.is_cancelled() {
                        return contact;
                    }
                    continue;
                }
            }
        }
        let after = applied.unwrap_or_default();
        tokio::select! {
            answer = client.fleet_watch(after) => match answer {
                Ok(generation) => {
                    contacted(local).await;
                    contact = Contact::Answered;
                    due = Some(generation) != *applied;
                }
                Err(error) if is_lost(&error) => {
                    tracing::info!(%error, "lost the fleet's home; reconnecting");
                    return contact;
                }
                Err(error) => {
                    let message = format!("watching the home: {error}");
                    tracing::warn!("{message}");
                    note_error(local, message).await;
                    due = match pause(pacing.backoff.delay(1), stop, &pacing.manual).await {
                        Wake::Manual => true,
                        Wake::Elapsed | Wake::Stopped => false,
                    };
                }
            },
            _tick = timer.tick() => due = true,
            () = pacing.manual.notified() => due = true,
            () = stop.cancelled() => return contact,
            () = client.closed() => return contact,
        }
    }
}

/// Whether the connection is gone (reconnect), rather than one call
/// failing.
const fn is_lost(error: &CallError) -> bool {
    matches!(error, CallError::Disconnected)
}

/// Fetch the manifest and the blobs this node lacks, apply, report.
async fn pull(client: &NodeClient, local: &Arc<Local>) -> Result<Generation, PullError> {
    let manifest = client.fleet_manifest().await?;
    contacted(local).await;
    let blobs = {
        let path = local.files.blobs();
        tokio::task::spawn_blocking(move || BlobStore::open(path))
            .await?
            .map_err(PullError::Blob)?
    };
    fetch(client, &manifest, &blobs).await?;
    let outcome = {
        let local = Arc::clone(local);
        let manifest = manifest.clone();
        let blobs = blobs.clone();
        tokio::task::spawn_blocking(move || -> Result<_, PullError> {
            files::write_json(&local.files.manifest(), &manifest)?;
            let outcome = local.apply(&manifest, &blobs, &[])?;
            let keep = manifest.files.values().map(|file| file.hash).collect();
            if let Err(error) = blobs.retain(&keep) {
                tracing::warn!(%error, "could not clean the fleet's blob store");
            }
            Ok(outcome)
        })
        .await??
    };
    client
        .fleet_report(manifest.generation, outcome.conflicts)
        .await?;
    Ok(manifest.generation)
}

/// Fetch every blob of `manifest` that `blobs` lacks, chunk by chunk.
async fn fetch(
    client: &NodeClient,
    manifest: &Manifest,
    blobs: &BlobStore,
) -> Result<(), PullError> {
    chunks::fetch(manifest.files.values(), blobs, |hash, offset| {
        client.fleet_blob(hash, offset)
    })
    .await
    .map_err(|error| match error {
        FetchError::Read(error) => PullError::Call(error),
        FetchError::Oversized(size) => PullError::Oversized(size),
        FetchError::Sink(error) => PullError::Blob(error),
        FetchError::Task(error) => PullError::Task(error),
    })
}

async fn note_error(local: &Arc<Local>, message: String) {
    let local = Arc::clone(local);
    let _written = tokio::task::spawn_blocking(move || {
        local.state.update(|state| state.error = Some(message));
    })
    .await;
}

/// Why the home did not take this node's grant on this connection, or
/// `None` once it did (or nothing was asked).
async fn note_grant(local: &Arc<Local>, refused: Option<String>) {
    let local = Arc::clone(local);
    let _written = tokio::task::spawn_blocking(move || {
        local.state.update(|state| state.grant = refused);
    })
    .await;
}

async fn contacted(local: &Arc<Local>) {
    let local = Arc::clone(local);
    let _written = tokio::task::spawn_blocking(move || {
        local
            .state
            .update(|state| state.contacted_at = Some(Utc::now()));
    })
    .await;
}

/// Wait `delay`, or until `stop`.
enum Wake {
    Elapsed,
    Manual,
    Stopped,
}

async fn pause(delay: Duration, stop: &CancellationToken, manual: &tokio::sync::Notify) -> Wake {
    tokio::select! {
        () = tokio::time::sleep(delay) => Wake::Elapsed,
        () = stop.cancelled() => Wake::Stopped,
        () = manual.notified() => Wake::Manual,
    }
}
