//! Disable acknowledgement follows group reaping, never just a sent signal.

use tokio::sync::mpsc;
use vessel_proc::{Child, Signal};
use vessel_proto::plugins::NodeToHelper;

use crate::{runtime_error::RuntimeError, runtime_types::SHUTDOWN_GRACE};

pub(crate) async fn reap(
    child: &mut Child,
    writer: &mpsc::Sender<NodeToHelper>,
) -> Result<(), RuntimeError> {
    // A full writer means cooperative shutdown could not be admitted. Do not
    // block on that queue or acknowledge disable while old callbacks run.
    if writer.try_send(NodeToHelper::Shutdown {}).is_ok() && exited(child).await? {
        return Ok(());
    }
    if let Err(source) = child.signal(Signal::Terminate) {
        // Still attempt the owned group's hard stop. Preserve the first signal
        // cause when this path cannot prove graceful group termination.
        let _hard_stop = child.signal(Signal::Kill);
        let _reaped = exited(child).await;
        return Err(RuntimeError::Signal(source));
    }
    if exited(child).await? {
        return Ok(());
    }
    child.signal(Signal::Kill).map_err(RuntimeError::Signal)?;
    if exited(child).await? {
        Ok(())
    } else {
        Err(RuntimeError::CleanupTimeout)
    }
}

async fn exited(child: &mut Child) -> Result<bool, RuntimeError> {
    match tokio::time::timeout(SHUTDOWN_GRACE, child.wait()).await {
        Ok(result) => result.map(|_code| true).map_err(RuntimeError::Wait),
        Err(_elapsed) => Ok(false),
    }
}
