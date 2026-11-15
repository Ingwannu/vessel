//! Inspect only a directory bound to the authenticated greeting. Warnings
//! precede the RPC because an older launchd job can restart immediately.
//! This slice executes no service manager and never rewrites a unit.

use crate::error::CliError;
use crate::init::{ServiceFile, ServiceManager, ServiceMatch};
use vessel_node::DataDir;

use super::local_read::{Kind, Pending};

/// Keep disk-template evidence distinct from a running manager's state.
pub(super) enum Observation {
    /// Remote, another directory, unsupported platform or no installed file.
    Absent,
    /// A disk template matches; no loaded-manager state has been inspected.
    Template,
    /// Preserve personal edits and show only trusted, computed guidance.
    Different,
}

/// Resolve the selected path before comparing with manager-independent arguments.
///
/// # Errors
/// The user's service file cannot be safely inspected.
pub(super) async fn inspect(directory: Option<&DataDir>) -> Result<Observation, CliError> {
    let Some(directory) = directory else {
        return Ok(Observation::Absent);
    };
    let Some(manager) = ServiceManager::current() else {
        return Ok(Observation::Absent);
    };
    let home = dirs::home_dir().ok_or(CliError::NoHome)?;
    let config = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from);
    let file = manager.file(&home, config.as_deref())?;
    let exe = std::env::current_exe()?;
    let data = std::path::absolute(directory.root())?;
    let mut pending = Pending::spawn(
        Kind::ServiceFile,
        move || read(manager, &file, &exe, &data),
        || tracing::debug!("node service observation completed after its command ended"),
    )?;
    Ok(pending.wait().await.map_err(CliError::ServiceReader)??)
}

fn read(
    manager: ServiceManager,
    file: &std::path::Path,
    exe: &std::path::Path,
    data: &std::path::Path,
) -> Result<Observation, crate::init::ServiceFileError> {
    let Some(contents) = ServiceFile::read(file)? else {
        return Ok(Observation::Absent);
    };
    Ok(match contents.classify(manager, file, exe, data) {
        ServiceMatch::Exact => Observation::Template,
        ServiceMatch::Different => Observation::Different,
    })
}

impl Observation {
    /// A warning is file evidence only; never promise a manager was stopped.
    pub(super) const fn warning(&self) -> Option<&'static str> {
        match self {
            Self::Absent | Self::Template => None,
            Self::Different => Some(
                "the user service differs from this version's template; left alone; the node may start again\n",
            ),
        }
    }
}
