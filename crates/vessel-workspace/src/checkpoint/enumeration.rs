//! Checkpoints cannot use the name index's lossy UTF-8/invalid-path skipping.

use super::Error;
use crate::{Git, git::Limits};
use std::{collections::BTreeSet, path::Path};
use vessel_proto::checkpoint::{
    CHECKPOINT_PATH_BYTES, CHECKPOINT_PATHS, CheckpointError, CheckpointPath, PathCount,
};

/// Includes a delimiter byte per path; a clipped listing always refuses.
const ENUMERATION_BYTES: usize =
    CHECKPOINT_PATHS.saturating_mul(CHECKPOINT_PATH_BYTES.saturating_add(1));

#[cfg(test)]
pub(super) async fn paths(git: &Git, root: &Path) -> Result<Vec<CheckpointPath>, Error> {
    cancellable(git, root, &tokio_util::sync::CancellationToken::new()).await
}

pub(super) async fn cancellable(
    git: &Git,
    root: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<Vec<CheckpointPath>, Error> {
    let output = git
        .command(root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .limits(Limits::LOCAL.with_stdout(ENUMERATION_BYTES))
        .cancel_on(cancellation.clone())
        .output()
        .await?;
    if output.stdout_cut {
        return Err(CheckpointError::TooManyPaths.into());
    }
    let output = output.checked()?;
    let mut unique = BTreeSet::new();
    for raw in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        let text = std::str::from_utf8(raw).map_err(|_invalid| CheckpointError::InvalidPath)?;
        let relative = text
            .parse::<vessel_proto::workspace::RelPath>()
            .map_err(|_invalid| CheckpointError::InvalidPath)?;
        let _new = unique.insert(CheckpointPath::try_from(relative)?);
        let _count = PathCount::new(unique.len())?;
    }
    Ok(unique.into_iter().collect())
}
