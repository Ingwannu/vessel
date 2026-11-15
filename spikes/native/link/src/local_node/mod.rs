//! One process-wide actor owns the desktop's local child across every window.
//!
//! [Decision Log]
//! - Purpose: closing one window must not drop a process another window uses.
//! - Constraint: GPUI has no Tokio reactor; the shared library owns child authority.
//! - Alternatives: a controller per window, or a mutex around a long-running call.
//! - Choice: a bounded actor on one process-owned runtime, with immutable watches.
//! - Cost: one reactor worker and a blocked supervisor retain runtime ownership
//!   through an irreversible OS exit; ordinary quit still finalizes beforehand.

mod actor;
mod shutdown;
mod signals;
pub use signals::QuitSignal;
pub mod registration;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use vessel_app_view::local_node::{AfterQuit, LocalProvenance, Management, Preferences, Status};
use vessel_client::NodeClient;
use vessel_local_node::{Launch, LocalNodeError, NodeHome, QuitPolicy, StopPolicy};

use crate::desktop::environment::ShellEnvironment;

pub use vessel_local_node::LogText;

const COMMAND_CAPACITY: usize = 8;

/// A launch recipe resolved only by the application executable edge.
#[derive(Debug)]
pub struct Configuration {
    launch: Option<Launch>,
    home: NodeHome,
    preferences: Preferences,
}

/// An actor refusal retains typed causes without command output or credentials.
#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    /// The configured home failed validation.
    #[error(transparent)]
    Files(#[from] vessel_local_node::FilesError),
    /// The fixed packaged launch recipe failed validation.
    #[error(transparent)]
    Launch(#[from] vessel_local_node::LaunchError),
    /// Runtime creation failed.
    #[error("local node runtime could not start")]
    Runtime(#[source] std::io::Error),
    /// The controller retains process ownership after a failed stop.
    #[error("local node control failed")]
    Control(#[source] Arc<LocalNodeError>, Box<Snapshot>),
    /// Missing companion binaries are an ordinary distribution state.
    #[error("local node is not installed")]
    NotInstalled,
    /// The controller stopped or cannot accept more work.
    #[error("local node command queue is unavailable")]
    Unavailable,
    /// The displayed process state changed after a confirmation was shown.
    #[error("local node confirmation is stale")]
    StaleConfirmation,
    /// The bounded log actor retained the original read cause.
    #[error("local node logs could not be read")]
    Logs(#[source] vessel_local_node::LogError),
}

impl ManagerError {
    /// The exact failed operation's observation, retained for a fixed confirmation.
    #[must_use]
    pub fn failure_snapshot(&self) -> Option<&Snapshot> {
        match self {
            Self::Control(_, snapshot) => Some(snapshot),
            Self::Files(_)
            | Self::Launch(_)
            | Self::Runtime(_)
            | Self::NotInstalled
            | Self::Unavailable
            | Self::StaleConfirmation
            | Self::Logs(_) => None,
        }
    }
}

impl Configuration {
    /// Resolve the fixed packaged binary, never a shell or PATH search.
    ///
    /// # Errors
    /// Relative paths or an environment hook changing the launch recipe.
    pub fn new(
        binary: PathBuf,
        home: PathBuf,
        environment: &ShellEnvironment,
        label: &str,
        preferences: Preferences,
    ) -> Result<Self, ManagerError> {
        let home = NodeHome::new(home)?;
        let launch = if binary.is_file() {
            Some(
                Launch::new(binary, home.clone(), crate::nodes::client_info(label))?
                    .environment(|command| environment.apply(command))?,
            )
        } else {
            None
        };
        Ok(Self {
            launch,
            home,
            preferences,
        })
    }
}

/// The only connection export is authenticated controller evidence, never a PID.
#[derive(Clone)]
pub struct Connection {
    client: NodeClient,
    provenance: LocalProvenance,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("node", &self.client.node().id)
            .field("provenance", &self.provenance)
            .finish_non_exhaustive()
    }
}

/// Actor snapshots are correlated by the current actual connection identity.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Exact evidence-backed state, translated by the screen.
    pub status: Status,
    /// Original failure, retained for diagnostics rather than parsing messages.
    pub failure: Option<Arc<LocalNodeError>>,
    connection: Option<Connection>,
    permit: Arc<()>,
}

impl Snapshot {
    /// Each window registers the same authenticated connection in its own link.
    #[must_use]
    pub fn connection(&self) -> Option<Connection> {
        self.connection.clone()
    }
    /// Capture while showing the explicit interruption choice.
    #[must_use]
    pub fn confirmation(&self) -> Option<Confirmation> {
        matches!(self.status, Status::Owned(_)).then(|| Confirmation(Arc::clone(&self.permit)))
    }
    /// The protected-stop refusal supplies actual session IDs to the confirmation UI.
    #[must_use]
    pub fn active_sessions(&self) -> Option<&[vessel_proto::SessionId]> {
        match self.failure.as_deref() {
            Some(LocalNodeError::ActiveSessions(sessions)) => Some(sessions),
            _ => None,
        }
    }
}

/// Opaque confirmation bound to one observed owned-node state, never forged by UI.
#[derive(Debug, Clone)]
pub struct Confirmation(Arc<()>);

/// Owned runtime is process-wide, never tied to a particular window's lifetime.
#[derive(Debug)]
pub struct Controller {
    commands: mpsc::Sender<actor::Command>,
    state: watch::Receiver<Snapshot>,
    reactor: tokio::runtime::Handle,
    shutdown: std::sync::mpsc::SyncSender<shutdown::Policy>,
    home: NodeHome,
}

impl Drop for Controller {
    fn drop(&mut self) {
        let _closed = self.shutdown.try_send(shutdown::Policy::Ordinary);
    }
}

/// Explicit user intent; interruption requires a distinct confirmed value.
#[derive(Debug, Clone)]
pub enum Action {
    /// Start or authenticate reuse at the managed home.
    Start,
    /// Stop only an owned child; live sessions are protected.
    Stop,
    /// Stop then await actual exit before launching a replacement.
    Restart,
    /// The person explicitly confirmed interrupting active sessions.
    InterruptConfirmed(Confirmation),
    /// Explicit interruption followed by a verified-exit replacement launch.
    RestartInterruptConfirmed(Confirmation),
}

impl Controller {
    /// Observe app quit signals once, on this process-owned reactor.
    #[must_use]
    pub fn quit_signals(&self) -> mpsc::Receiver<Result<QuitSignal, std::io::Error>> {
        let (sender, receiver) = mpsc::channel(1);
        drop(self.reactor.spawn(signals::observe(sender)));
        receiver
    }

    /// Irrevocable OS exit cannot wait for the ordinary protected stop dialog.
    /// Transfer runtime lifetime before GPUI's short shutdown callback expires.
    /// This neither interrupts sessions nor changes the saved quit preference.
    ///
    /// # Errors
    /// The supervisor has already ended; no cleanup acknowledgement is implied.
    pub fn preserve_on_exit(&self) -> Result<(), ManagerError> {
        match self.shutdown.try_send(shutdown::Policy::Preserve) {
            Ok(()) | Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => Err(ManagerError::Unavailable),
        }
    }

    /// The explicit managed home's existing private log, never a remote path.
    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.home.log_path()
    }

    /// Explicit read of the bounded tail, executed on the controller reactor.
    ///
    /// # Errors
    /// No owned log source, bounded reader failure, or closed command queue.
    pub fn log_tail(
        &self,
    ) -> impl Future<Output = Result<LogText, ManagerError>> + 'static + use<> {
        let commands = self.commands.clone();
        async move {
            let (reply, wait) = oneshot::channel();
            commands
                .send(actor::Command::Logs(reply))
                .await
                .map_err(|_closed| ManagerError::Unavailable)?;
            wait.await.map_err(|_closed| ManagerError::Unavailable)?
        }
    }

    /// Start one actor; ordinary missing binaries remain a visible status.
    ///
    /// # Errors
    /// The runtime could not be created.
    pub fn start(configuration: Configuration) -> Result<Self, ManagerError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("vessel-local-node")
            .enable_all()
            .build()
            .map_err(ManagerError::Runtime)?;
        let initial = if configuration.launch.is_none() {
            Status::NotInstalled
        } else if configuration.preferences.management == Management::Paused {
            Status::Paused
        } else {
            Status::Working
        };
        let (published, state) = watch::channel(Snapshot {
            status: initial,
            failure: None,
            connection: None,
            permit: Arc::new(()),
        });
        let (commands, inbox) = mpsc::channel(COMMAND_CAPACITY);
        let home = configuration.home.clone();
        let reactor = runtime.handle().clone();
        let (shutdown, policy) = std::sync::mpsc::sync_channel(shutdown::CAPACITY);
        let keep_commands = commands.clone();
        let keep_state = state.clone();
        let supervisor = std::thread::Builder::new()
            .name("vessel-node-exit".to_owned())
            .spawn(move || {
                // No child can start before supervisor creation succeeds.
                drop(runtime.spawn(actor::run(configuration, inbox, published)));
                shutdown::run(runtime, &policy, keep_commands, keep_state);
            })
            .map_err(ManagerError::Runtime)?;
        drop(supervisor);
        Ok(Self {
            commands,
            state,
            reactor,
            shutdown,
            home,
        })
    }

    /// Subscribe without another node launch or runtime.
    #[must_use]
    pub fn state(&self) -> watch::Receiver<Snapshot> {
        self.state.clone()
    }

    /// Synchronous UI admission never blocks the main thread.
    ///
    /// # Errors
    /// The bounded actor queue is full or closed.
    pub fn request(
        &self,
        action: Action,
    ) -> Result<impl Future<Output = Result<Snapshot, ManagerError>> + 'static + use<>, ManagerError>
    {
        let (reply, wait) = oneshot::channel();
        self.commands
            .try_send(actor::Command::Action { action, reply })
            .map_err(|_closed| ManagerError::Unavailable)?;
        Ok(async move { wait.await.map_err(|_closed| ManagerError::Unavailable)? })
    }

    /// Capture this future before leaving the GPUI update closure.
    ///
    /// # Errors
    /// Active sessions, actual stop failure, or actor closure; ownership is retained.
    pub fn finalize(
        &self,
        after: AfterQuit,
        confirmation: Option<Confirmation>,
    ) -> impl Future<Output = Result<(), ManagerError>> + 'static + use<> {
        let commands = self.commands.clone();
        let policy = match after {
            AfterQuit::KeepRunning => QuitPolicy::KeepRunning,
            AfterQuit::Stop => QuitPolicy::Stop(if confirmation.is_some() {
                StopPolicy::InterruptConfirmed
            } else {
                StopPolicy::Protect
            }),
        };
        async move {
            let (reply, wait) = oneshot::channel();
            commands
                .send(actor::Command::Finalize {
                    policy,
                    confirmation,
                    reply,
                })
                .await
                .map_err(|_closed| ManagerError::Unavailable)?;
            wait.await.map_err(|_closed| ManagerError::Unavailable)?
        }
    }
}
