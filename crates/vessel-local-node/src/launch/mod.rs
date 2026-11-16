//! The only Proc consumer: direct owned handles, never record-derived PIDs.

use std::path::PathBuf;
use vessel_proc::CommandSpec;
use vessel_proto::ClientInfo;

use crate::NodeHome;

mod owned;
pub use owned::{KeptNode, ProcessExit};
pub(crate) use owned::{Owned, START_DEADLINE, STOP_DEADLINE, Started};

/// A validated launch recipe, with caller-resolved environment applied to the child only.
#[derive(Debug, Clone)]
pub struct Launch {
    command: CommandSpec,
    home: NodeHome,
    client: ClientInfo,
}

/// Launch validation and owned process failures preserve their original cause.
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// Persistent identity or a live child alone cannot qualify an RPC peer.
    #[error("local node peer is not the retained launched process")]
    PeerUnverified,
    /// The retained child stdout may contain exactly one managed first record.
    #[error("local node emitted extra startup output")]
    ExtraReadiness,
    /// OS randomness failed; no weaker name or challenge is substituted.
    #[error("local launch entropy is unavailable")]
    Entropy(#[source] getrandom::Error),
    /// The private bootstrap record or stdout echo was malformed.
    #[error("invalid private launch transcript")]
    Transcript(#[source] vessel_proto::local_launch::TranscriptError),
    /// Opening or writing this private handle failed, without retaining its name.
    #[error("private local launch connection failed")]
    Bootstrap(#[source] vessel_client::LocalConnectError),
    /// A child could not be started; its process owner retains cleanup.
    #[error("local node process could not start")]
    Spawn(#[source] vessel_proc::SpawnError),
    /// The dedicated readiness pipe failed.
    #[error("local node readiness pipe failed")]
    Read(#[source] std::io::Error),
    /// The one inherited child bootstrap record could not be written.
    #[error("local child bootstrap pipe write failed")]
    Write(#[source] std::io::Error),
    /// The bounded child record was malformed or absent.
    #[error("local node did not emit valid readiness")]
    Ready(#[source] vessel_proto::local_launch::ReadyError),
    /// Platform range cannot represent the named protocol limit.
    #[error("local readiness read bound is unsupported")]
    ReadLimit,
    /// Leader exit or the first original-group cleanup request was not confirmed.
    #[error("local node owned cleanup was not confirmed")]
    Wait(#[source] vessel_proc::CheckedWaitError),
    /// Release capability was refused; the controller retains its owned child.
    /// Proc's refusal has only this policy cause; its recovered Child stays internal.
    #[error("local node release capability was refused")]
    ReleaseCapability,
    /// Observation after release failed; no signal authority is recreated.
    #[error("kept local node could not be observed")]
    DetachedWait(#[source] vessel_proc::DetachedError),
    /// Executables must be resolved outside the library before a launch.
    #[error("local node executable must be absolute")]
    RelativeExecutable,
    /// A login-shell environment hook may change only environment, never argv/program.
    #[error("local node environment hook changed the launch recipe")]
    ChangedRecipe,
}

impl Launch {
    /// Construct the literal, credential-free child command.
    ///
    /// # Errors
    /// Relative executable paths are refused before any spawn.
    pub fn new(binary: PathBuf, home: NodeHome, client: ClientInfo) -> Result<Self, LaunchError> {
        if !binary.is_absolute() {
            return Err(LaunchError::RelativeExecutable);
        }
        let command = CommandSpec::new(binary)
            .args(["--desktop-managed", "--bind", "none", "--data-dir"])
            .arg(home.path());
        Ok(Self {
            command,
            home,
            client,
        })
    }

    /// Apply the desktop's already-resolved login environment without global mutation.
    ///
    /// # Errors
    /// Program, arguments and working directory must remain identical.
    pub fn environment(
        mut self,
        apply: impl FnOnce(CommandSpec) -> CommandSpec,
    ) -> Result<Self, LaunchError> {
        let updated = apply(self.command.clone());
        if updated.program != self.command.program
            || updated.args != self.command.args
            || updated.cwd != self.command.cwd
        {
            return Err(LaunchError::ChangedRecipe);
        }
        self.command = updated;
        Ok(self)
    }

    /// Explicit node home shared with a local CLI controller.
    #[must_use]
    pub const fn home(&self) -> &NodeHome {
        &self.home
    }

    /// Client identity used for the post-readiness authenticated greeting.
    #[must_use]
    pub const fn client(&self) -> &ClientInfo {
        &self.client
    }
}
