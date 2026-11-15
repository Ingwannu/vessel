//! Retain leader-reap and first-cleanup evidence, never claim descendant death.

use tokio::sync::mpsc;
use vessel_proc::{Child, OwnedGroupExit, Signal};
use vessel_proto::plugins::NodeToHelper;

use crate::{runtime_error::RuntimeError, runtime_types::SHUTDOWN_GRACE};

pub(crate) async fn reap(
    child: &mut Child,
    writer: &mpsc::Sender<NodeToHelper>,
) -> Result<OwnedGroupExit, RuntimeError> {
    // A full writer means cooperative shutdown could not be admitted. Do not
    // block on that queue or acknowledge disable while old callbacks run.
    if writer.try_send(NodeToHelper::Shutdown {}).is_ok()
        && let Some(receipt) = exited(child).await?
    {
        return Ok(receipt);
    }
    if let Err(source) = child.signal(Signal::Terminate) {
        // Still attempt the owned group's hard stop. Preserve the first signal
        // cause when this path cannot prove graceful group termination.
        let _hard_stop = child.signal(Signal::Kill);
        let _reaped = exited(child).await;
        return Err(RuntimeError::Signal(source));
    }
    if let Some(receipt) = exited(child).await? {
        return Ok(receipt);
    }
    child.signal(Signal::Kill).map_err(RuntimeError::Signal)?;
    exited(child).await?.ok_or(RuntimeError::CleanupTimeout)
}

async fn exited(child: &mut Child) -> Result<Option<OwnedGroupExit>, RuntimeError> {
    match tokio::time::timeout(SHUTDOWN_GRACE, child.wait_checked()).await {
        Ok(result) => result.map(Some).map_err(RuntimeError::Cleanup),
        Err(_elapsed) => Ok(None),
    }
}
