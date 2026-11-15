//! Recovery reconstructs expected identities from durable evidence, not current files.
use super::{Error, FileImage, FileState, Mode, PathCount, Prepared, Step, original::Original};
use vessel_proto::checkpoint::{CheckpointError, EffectiveDesiredImage, StagedPath, StagedPaths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Reconstructed per-path observation; the node still verifies the whole scope.
pub enum RecoveryState {
    /// Original state and inode remain; no undo is needed.
    Original,
    /// Our written state and inode remain; guarded undo may proceed.
    Desired,
    /// Neither expected image/object matches; preserve current data.
    Conflict,
}

impl Prepared {
    pub(in crate::checkpoint) fn recovery_state(
        &self,
        root: &super::Root,
        index: PathCount,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RecoveryState, Error> {
        let step = self
            .steps
            .get(index.get())
            .ok_or(CheckpointError::InvalidPath)?;
        let current = super::inspect::path(root, &step.path, cancel)?;
        let object = root
            .entry_identity(&step.path)?
            .map(super::original::object);
        if current == step.before && object == step.before_identity.object() {
            Ok(RecoveryState::Original)
        } else if current == step.desired && object == step.desired_identity {
            Ok(RecoveryState::Desired)
        } else {
            Ok(RecoveryState::Conflict)
        }
    }
    /// The node commits this complete set before the first project mutation.
    pub(in crate::checkpoint) fn evidence(&self) -> Result<StagedPaths, Error> {
        Ok(StagedPaths::new(
            self.steps
                .iter()
                .map(|step| StagedPath {
                    path: step.path.clone(),
                    before: step.before_identity.object(),
                    desired: step.desired_identity,
                })
                .collect(),
        )?)
    }

    /// This can only undo or inspect. It cannot resume forward application after
    /// a restart, when the fresh full observations no longer establish admission.
    pub(in crate::checkpoint) fn recovering(
        before: &FileImage,
        desired: &EffectiveDesiredImage,
        staged: &StagedPaths,
    ) -> Result<Self, Error> {
        let changes = desired.changes(before);
        if changes.len() != staged.entries().len() {
            return Err(CheckpointError::RecoveryPending.into());
        }
        let mut steps = Vec::new();
        for (index, (change, evidence)) in changes.into_iter().zip(staged.entries()).enumerate() {
            let original = before
                .get(&change.path)
                .cloned()
                .unwrap_or(FileState::Absent);
            let target = desired
                .image()
                .get(&change.path)
                .cloned()
                .ok_or(CheckpointError::RecoveryPending)?;
            if change.path != evidence.path
                || matches!(original, FileState::Present { .. }) != evidence.before.is_some()
                || matches!(target, FileState::Present { .. }) != evidence.desired.is_some()
            {
                return Err(CheckpointError::RecoveryPending.into());
            }
            steps.push(Step {
                path: change.path,
                before: original,
                desired: target,
                before_identity: evidence
                    .before
                    .map_or(Original::Absent, Original::Recovered),
                desired_identity: evidence.desired,
                index: PathCount::new(index)?,
            });
        }
        Ok(Self {
            steps,
            mode: Mode::Recovery,
        })
    }
}
