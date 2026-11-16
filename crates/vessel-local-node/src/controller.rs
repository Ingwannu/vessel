//! Serialized owned-child lifecycle. Reuse never becomes process authority.

use vessel_client::{CallError, LocalConnectError, NodeClient};
use vessel_proto::{NodeId, SessionId, StopWhen};

use crate::launch::{Owned, STOP_DEADLINE};

mod startup;
use crate::{FilesError, KeptNode, Launch, LaunchError, LogReader, ProcessExit};

/// Named operations make deadline errors meaningful without raw command output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Readiness pipe plus authenticated local greeting.
    Start,
    /// Graceful request plus actual owned-child exit.
    Stop,
}

/// A caller explicitly selects interruption; Protect never sends an implicit Now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopPolicy {
    /// Refuse active sessions; drain rather than race a new session into Now.
    Protect,
    /// Existing sessions finish naturally, with a finite caller-visible wait.
    Drain,
    /// The UI has explicitly confirmed interrupting this node's active sessions.
    InterruptConfirmed,
}

/// Quit's keep-running choice is distinct from manual stop confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitPolicy {
    /// Consume owned cleanup authority into an observation-only reaper.
    KeepRunning,
    /// Stop through an explicitly selected protection/interruption policy.
    Stop(StopPolicy),
}

/// Status records evidence, rather than equating socket closure with process exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalStatus {
    /// This controller launched and still owns the child.
    Owned(NodeId),
    /// Authenticated reuse, without stop/restart authority.
    Attached(NodeId),
    /// Explicit release retained observation, never signal authority.
    Kept(NodeId),
    /// The exact direct child was reaped.
    Exited(ProcessExit),
    /// A reused node's socket closed; its OS process state is unknown.
    AttachedDisconnected(NodeId),
}

/// Every refusal keeps the original cause; no readiness/log payload is printed.
#[derive(Debug, thiserror::Error)]
pub enum LocalNodeError {
    /// Private node files or start admission failed.
    #[error(transparent)]
    Files(#[from] FilesError),
    /// Owned launch/readiness/wait failed.
    #[error(transparent)]
    Launch(#[from] LaunchError),
    /// Local authentication or transport failed.
    #[error("local node authentication failed")]
    Connect(#[source] LocalConnectError),
    /// The typed node call failed.
    #[error("local node control request failed")]
    Call(#[source] CallError),
    /// An operation exceeded the reference's named deadline.
    #[error("local node {operation:?} timed out")]
    Deadline {
        /// Which operation remains unconfirmed.
        operation: Operation,
        /// Original timer cause.
        #[source]
        source: tokio::time::error::Elapsed,
    },
    /// A readiness record cannot replace the private persisted identity.
    #[error("local node identity was not created")]
    MissingIdentity,
    /// The authenticated socket or child names a different node.
    #[error("local node identity changed")]
    Identity {
        /// Previously intended node.
        expected: NodeId,
        /// Identity actually observed.
        found: NodeId,
    },
    /// A draining node is not ready for a fresh desktop session.
    #[error("local node is draining")]
    Draining,
    /// Token/PID/connection reuse never authorizes process cleanup.
    #[error("the local node is not owned by this controller")]
    NotOwned,
    /// The node has live work; a UI must explicitly confirm interruption.
    #[error("local node still has active sessions")]
    ActiveSessions(Vec<SessionId>),
    /// Original startup failed and its checked cleanup could not be observed.
    /// The exact owned Child is dropped before this is returned: its hard kill
    /// was requested, but no actual reap/group receipt is claimed.
    #[error("local node startup failed; original child hard kill requested but not observed")]
    CleanupUnconfirmed {
        /// The original startup error is retained without remapping it.
        #[source]
        start: Box<Self>,
        /// Graceful request, checked wait or its named deadline cause.
        cleanup: LaunchError,
    },
    /// An owned child exited during its authenticated readiness check.
    #[error("local node exited during startup")]
    StartupExit(ProcessExit),
}

enum Lease {
    Owned {
        client: NodeClient,
        child: Box<Owned>,
    },
    Attached(NodeClient),
    Kept {
        node: NodeId,
        observer: KeptNode,
    },
    Exited(ProcessExit),
    AttachedDisconnected(NodeId),
}

/// A caller's task owns this value and serializes its commands; no shared mutex.
///
/// Unix owned launch installs a SIGCHLD listener before creating the child and
/// retains it for this controller's lifetime. Embeddings must not replace that
/// disposition afterward: automatic reaping would invalidate retained PID proof.
pub struct LocalNode {
    launch: Launch,
    lease: Lease,
    logs: Option<LogReader>,
    #[cfg(unix)]
    _reaping: Option<tokio::signal::unix::Signal>,
}

struct Opened {
    lease: Lease,
    logs: Option<LogReader>,
    #[cfg(unix)]
    reaping: Option<tokio::signal::unix::Signal>,
}

impl std::fmt::Debug for LocalNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNode")
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}

impl LocalNode {
    /// Reuse an authenticated existing node as Attached, otherwise launch one.
    ///
    /// # Errors
    /// Admission, files, readiness, identity, authentication or startup deadline.
    pub async fn start(launch: Launch) -> Result<Self, LocalNodeError> {
        Self::start_observed(launch, |_logs| {}).await
    }

    /// Retain the bounded stderr reader before readiness, so a startup failure
    /// can offer diagnostics without weakening child cleanup or exposing logs
    /// in the error itself. The observer receives no process-control authority.
    ///
    /// # Errors
    /// The same admission, authentication, readiness and deadline causes as start.
    pub async fn start_observed(
        launch: Launch,
        on_logs: impl FnOnce(LogReader),
    ) -> Result<Self, LocalNodeError> {
        let opened = startup::open(&launch, on_logs).await?;
        Ok(Self {
            launch,
            lease: opened.lease,
            logs: opened.logs,
            #[cfg(unix)]
            _reaping: opened.reaping,
        })
    }

    /// The current evidence-backed status.
    #[must_use]
    pub fn status(&self) -> LocalStatus {
        match &self.lease {
            Lease::Owned { client, .. } => LocalStatus::Owned(client.node().id),
            Lease::Attached(client) => LocalStatus::Attached(client.node().id),
            Lease::Kept { node, .. } => LocalStatus::Kept(*node),
            Lease::Exited(exit) => LocalStatus::Exited(*exit),
            Lease::AttachedDisconnected(node) => LocalStatus::AttachedDisconnected(*node),
        }
    }

    /// Hand a clone to the app transport only after authenticated readiness.
    #[must_use]
    pub fn client(&self) -> Option<&NodeClient> {
        match &self.lease {
            Lease::Owned { client, .. } | Lease::Attached(client) => Some(client),
            Lease::Kept { .. } | Lease::Exited(_) | Lease::AttachedDisconnected(_) => None,
        }
    }

    /// The bounded log tail remains available after an owned stop.
    #[must_use]
    pub fn logs(&self) -> Option<LogReader> {
        self.logs.clone()
    }

    /// Request stop only for an owned child, then await that exact child's exit.
    /// A timeout retains ownership and never reports Stopped.
    ///
    /// # Errors
    /// Not owned, unconfirmed active sessions, RPC/wait failure or deadline.
    pub async fn stop(&mut self, policy: StopPolicy) -> Result<ProcessExit, LocalNodeError> {
        let exit = tokio::time::timeout(STOP_DEADLINE, stop(&mut self.lease, policy))
            .await
            .map_err(|source| LocalNodeError::Deadline {
                operation: Operation::Stop,
                source,
            })??;
        self.lease = Lease::Exited(exit);
        Ok(exit)
    }

    /// A replacement launch starts only after real owned-child exit.
    /// A concurrent external winner is returned explicitly as Attached.
    ///
    /// # Errors
    /// Any stop, start, authentication or readiness failure.
    pub async fn restart(&mut self, policy: StopPolicy) -> Result<LocalStatus, LocalNodeError> {
        self.restart_observed(policy, |_logs| {}).await
    }

    /// Retain replacement startup diagnostics even if its readiness later fails.
    /// The old owned process must exit before this observer can receive a reader.
    ///
    /// # Errors
    /// The same protected stop, actual exit, startup or readiness failures as restart.
    pub async fn restart_observed(
        &mut self,
        policy: StopPolicy,
        on_logs: impl FnOnce(LogReader),
    ) -> Result<LocalStatus, LocalNodeError> {
        let _exit = self.stop(policy).await?;
        let replacement = Self::start_observed(self.launch.clone(), on_logs).await?;
        *self = replacement;
        Ok(self.status())
    }

    /// Finalize an owned node; Attached reuse is always left running.
    ///
    /// # Errors
    /// Owned stop/release failed; a timed-out stop keeps its owned handle alive.
    pub async fn quit(&mut self, policy: QuitPolicy) -> Result<LocalStatus, LocalNodeError> {
        if !matches!(self.lease, Lease::Owned { .. }) {
            return Ok(self.status());
        }
        match policy {
            QuitPolicy::Stop(policy) => {
                let _exit = self.stop(policy).await?;
            }
            QuitPolicy::KeepRunning => {
                let node = self.client().ok_or(LocalNodeError::NotOwned)?.node().id;
                // Lease is temporarily replaced only inside this synchronous
                // consume; no caller observes a false exit or another owner.
                let lease = std::mem::replace(&mut self.lease, Lease::AttachedDisconnected(node));
                if let Lease::Owned { client, child } = lease {
                    match (*child).keep() {
                        Ok(observer) => {
                            self.lease = Lease::Kept { node, observer };
                        }
                        Err(child) => {
                            self.lease = Lease::Owned { client, child };
                            return Err(LaunchError::ReleaseCapability.into());
                        }
                    }
                }
            }
        }
        Ok(self.status())
    }

    /// Observe exit without polling; closure of an Attached connection carries
    /// no OS-process claim. The caller bounds an operation when necessary.
    ///
    /// # Errors
    /// Waiting on the exact owned/released child failed.
    pub async fn wait_exit(&mut self) -> Result<LocalStatus, LocalNodeError> {
        let next = match &mut self.lease {
            Lease::Owned { child, .. } => Lease::Exited(child.wait().await?),
            Lease::Kept { observer, .. } => Lease::Exited(observer.wait().await?),
            Lease::Attached(client) => {
                client.closed().await;
                Lease::AttachedDisconnected(client.node().id)
            }
            Lease::Exited(_) | Lease::AttachedDisconnected(_) => return Ok(self.status()),
        };
        self.lease = next;
        Ok(self.status())
    }
}

async fn stop(lease: &mut Lease, policy: StopPolicy) -> Result<ProcessExit, LocalNodeError> {
    let (client, child) = match lease {
        Lease::Owned { client, child } => (client, child),
        Lease::Exited(exit) => return Ok(*exit),
        Lease::Attached(_) | Lease::Kept { .. } | Lease::AttachedDisconnected(_) => {
            return Err(LocalNodeError::NotOwned);
        }
    };
    let when = match policy {
        StopPolicy::Protect => {
            let live: Vec<_> = client
                .sessions()
                .await
                .map_err(LocalNodeError::Call)?
                .into_iter()
                .filter(|session| session.status.is_live())
                .map(|session| session.id)
                .collect();
            if !live.is_empty() {
                return Err(LocalNodeError::ActiveSessions(live));
            }
            StopWhen::WhenIdle
        }
        StopPolicy::Drain => StopWhen::WhenIdle,
        StopPolicy::InterruptConfirmed => StopWhen::Now,
    };
    match client.stop_node(when).await {
        Ok(()) | Err(CallError::Disconnected) => {}
        Err(error) => return Err(LocalNodeError::Call(error)),
    }
    Ok(child.wait().await?)
}
