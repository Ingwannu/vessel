//! One approved port-helper invocation, staged until allocator wiring removes
//! the outer test-only declaration. No planning, approval or health policy here.

mod capture;
mod program;

use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use vessel_proc::CommandSpec;
use vessel_proto::WorkspaceId;
use vessel_proto::scripts::ScriptName;
use vessel_proto::workspace::{InvalidServicePort, PortScript, ServicePort};
use vessel_workspace::paths::PathError;

use crate::terminals::worker_failure::WorkerFailure;
use crate::workspace::ScriptRoot;

/// Bound the complete invocation, including checked path resolution.
const TIMEOUT: Duration = Duration::from_secs(10);
/// `execFile` limits each output pipe independently, not their sum.
const MAX_OUTPUT_BYTES: u16 = 1024;

/// A fixed refusal without helper output or arguments in its diagnostic.
#[derive(Debug, thiserror::Error)]
pub(super) enum PortScriptError {
    #[error("the port-script path cannot be interpreted on this platform")]
    Platform,
    #[error(
        "a port script on Windows must be a native executable, not a .cmd or .bat command script"
    )]
    WindowsCommandScript,
    #[error(
        "a port-script path on Windows cannot use alternate streams or trailing dots or spaces"
    )]
    WindowsPath,
    #[error("the resolved port-script path is not Unicode on Windows")]
    NativePath,
    #[error("the port script must name a regular file")]
    NotFile,
    #[error("the port-script checkout or relative path cannot be resolved")]
    Path(#[source] PathError),
    #[error("the port-script path cannot be resolved")]
    Resolve(#[source] io::Error),
    #[error("resolving the port-script path stopped")]
    Join(#[source] WorkerFailure),
    #[error("the port script could not be started")]
    Spawn(#[source] vessel_proc::SpawnError),
    #[error("waiting for the port script failed")]
    Wait(#[source] vessel_proc::WaitError),
    #[error("the port script failed with exit code {0:?}")]
    Exit(Option<i32>),
    #[error("reading the port script's {stream} failed")]
    Read {
        stream: vessel_proc::Stream,
        #[source]
        source: io::Error,
    },
    #[error("the port script's {0} exceeded {max} bytes", max = MAX_OUTPUT_BYTES)]
    OutputLimit(vessel_proc::Stream),
    #[error("the port script's stdout is not UTF-8")]
    Utf8(#[source] std::str::Utf8Error),
    #[error("the port script must print one TCP port")]
    Port(#[source] InvalidServicePort),
    #[error("the port script exceeded {seconds} seconds", seconds = TIMEOUT.as_secs())]
    Timeout,
    #[error("the port-script invocation was cancelled")]
    Cancelled,
}

/// Execute one owner selected by an approval-checked caller, without a shell.
pub(super) async fn invoke(
    workspace: WorkspaceId,
    root: &ScriptRoot,
    name: &ScriptName,
    program: &PortScript,
    removed: &[OsString],
    cancel: &CancellationToken,
) -> Result<ServicePort, PortScriptError> {
    let work = async {
        let spec = prepare(workspace, root, name, program, removed).await?;
        // A cancellation during blocking path resolution must not start a child.
        if cancel.is_cancelled() {
            return Err(PortScriptError::Cancelled);
        }
        let spawned = vessel_proc::spawn(&spec).map_err(PortScriptError::Spawn)?;
        capture::port(spawned).await
    };
    bounded(work, cancel).await
}

/// Dropping the selected work also drops its child owner and kills the group/job.
async fn bounded(
    work: impl Future<Output = Result<ServicePort, PortScriptError>>,
    cancel: &CancellationToken,
) -> Result<ServicePort, PortScriptError> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(PortScriptError::Cancelled),
        result = tokio::time::timeout(TIMEOUT, work) => {
            result.map_err(|_elapsed| PortScriptError::Timeout)?
        }
    }
}

/// Build the literal argument/env boundary from the node's typed placement.
async fn prepare(
    workspace: WorkspaceId,
    root: &ScriptRoot,
    name: &ScriptName,
    program: &PortScript,
    removed: &[OsString],
) -> Result<CommandSpec, PortScriptError> {
    let folder = root.root.clone();
    let program = program.clone();
    let (program, folder) =
        tokio::task::spawn_blocking(move || program::resolve(&folder, &program))
            .await
            .map_err(|source| PortScriptError::Join(source.into()))??;
    let branch = root.branch.as_ref().map_or("", |branch| branch.as_str());
    let values = [
        ("VESSEL_SCRIPT_NAME", OsString::from(name.as_str())),
        ("VESSEL_WORKSPACE_ID", OsString::from(workspace.to_string())),
        ("VESSEL_BRANCH", OsString::from(branch)),
        ("VESSEL_WORKTREE", folder.clone().into_os_string()),
    ];
    let mut spec = CommandSpec::new(program).cwd(folder);
    for name in removed {
        spec = spec.env_remove(name);
    }
    for (name, value) in values {
        spec = spec.arg(value.clone()).env(name, value);
    }
    Ok(spec)
}

#[cfg(test)]
mod tests;
