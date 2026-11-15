//! The admitted worker re-enumerates coverage and every effective ignore input.
use super::{
    Error,
    archive::Archive,
    capture, enumeration,
    policy::{Defaults, Inputs},
    rooted::Root,
};
use crate::Git;
use std::path::Path;
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{CheckpointError, ObservedSnapshot};

/// Run only on the existing admitted blocking worker, never on an async thread.
/// The explicit runtime only drives bounded Git subprocess I/O between file reads;
/// it does not spawn another unbounded filesystem worker or read ambient HOME.
#[cfg(test)]
pub(super) fn capture(
    runtime: &tokio::runtime::Handle,
    git: &Git,
    root: &Path,
    archive: &Archive,
    defaults: &Defaults,
    cancel: &CancellationToken,
) -> Result<ObservedSnapshot, Error> {
    let opened = Root::open(root)?;
    capture_opened(runtime, git, root, &opened, archive, defaults, cancel)
}

pub(super) fn capture_opened(
    runtime: &tokio::runtime::Handle,
    git: &Git,
    root: &Path,
    opened: &Root,
    archive: &Archive,
    defaults: &Defaults,
    cancel: &CancellationToken,
) -> Result<ObservedSnapshot, Error> {
    opened.revalidate()?;
    let before = runtime.block_on(enumeration::cancellable(git, root, cancel))?;
    let inputs = runtime.block_on(Inputs::discover(git, root, &before, defaults, cancel))?;
    let policy = inputs.fingerprint(opened, cancel)?;
    let snapshot = capture::observed(opened, archive, &before, policy, cancel)?;
    let after = runtime.block_on(enumeration::cancellable(git, root, cancel))?;
    let rechecked = runtime.block_on(Inputs::discover(git, root, &after, defaults, cancel))?;
    if before != after || inputs != rechecked || rechecked.fingerprint(opened, cancel)? != policy {
        return Err(CheckpointError::CaptureUnstable.into());
    }
    opened.revalidate()?;
    Ok(snapshot)
}
#[cfg(test)]
mod tests;
