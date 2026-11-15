//! Desktop-local I/O takes explicit paths from the binary, never node credentials.

pub mod cli;
pub mod environment;
pub mod instance;
pub mod invocation;
pub mod preferences;
pub mod store;
pub mod window_writer;

/// Refusals preserve their source without retaining environment or file contents.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The operation needs a trusted absolute path from the host entry point.
    #[error("desktop paths must be absolute")]
    RelativePath,
    /// A bundled tool cannot be invented when the release did not contain one.
    #[error("the bundled Vessel CLI is unavailable in this installation")]
    MissingCli,
    /// Installation must never replace a tool owned by another installation.
    #[error("the CLI destination belongs to another installation")]
    ForeignCli,
    /// Platform operations keep the original OS error.
    #[error("desktop file operation failed")]
    Io(#[from] std::io::Error),
    /// Invalid settings do not include raw file contents in the error.
    #[error("desktop settings are invalid")]
    Json(#[from] serde_json::Error),
    /// Newer documents must survive a launch of an older app.
    #[error("desktop settings were written by a newer application")]
    NewerDocument,
    /// A bounded desktop worker cannot accept more changes at this moment.
    #[error("the desktop worker queue is full")]
    Busy,
    /// A closed worker cannot promise persistence.
    #[error("the desktop worker has stopped")]
    Closed,
    /// A launch request exceeded the local protocol boundary.
    #[error("invalid desktop launch request")]
    InvalidLaunch,
    /// A primary process has not acknowledged the request within the deadline.
    #[error("the running desktop application did not acknowledge the launch")]
    LaunchDeadline,
    /// A login shell did not return a bounded, well-formed environment.
    #[error("the login shell environment could not be resolved")]
    ShellEnvironment,
    /// Process primitives retain ownership of process-group cleanup.
    #[error("the login shell could not start")]
    Spawn(#[from] vessel_proc::SpawnError),
    /// Waiting retains the original process failure.
    #[error("the login shell could not be reaped")]
    Wait(#[from] vessel_proc::WaitError),
    /// Merging independent edits still respects the preference model's bounds.
    #[error("the preference change exceeds its limit")]
    Preference(#[from] vessel_app_view::prefs::Refused),
    /// The existing preferences writer remains the only persistence path.
    #[error("application preferences could not be saved")]
    PreferencesWrite(#[source] crate::LinkError),
}
