//! Per-path apply/undo only. The node owns durable journal ordering and final status.
//!
//! All desired regular files are staged before the first project mutation.
//! Existing replacements exchange inodes, retaining the original inside the
//! journal. Rollback refuses an external change rather than overwriting it.
pub(super) mod stage;
#[cfg(test)]
mod tests;

use super::{
    Error,
    archive::Archive,
    inspect,
    rooted::{Root, mutation::ParentPolicy},
};
use stage::{Slot, SlotKind, Stage};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CheckpointError, CheckpointPath, EffectiveDesiredImage, FileImage, FileState, PathCount,
};

#[derive(Debug, Clone)]
pub(super) struct Step {
    pub(super) path: CheckpointPath,
    before: FileState,
    desired: FileState,
    index: PathCount,
}
#[derive(Debug)]
pub(super) struct Prepared {
    steps: Vec<Step>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Undo {
    Restored,
    Conflict,
}
impl Prepared {
    pub(super) fn new(
        root: &Root,
        archive: &Archive,
        stage: &Stage,
        before: &FileImage,
        desired: &EffectiveDesiredImage,
        cancellation: &CancellationToken,
    ) -> Result<Self, Error> {
        let changes = desired.changes(before);
        let mut steps = Vec::new();
        for (index, change) in changes.into_iter().enumerate() {
            let index = PathCount::new(index)?;
            let before = before
                .get(&change.path)
                .cloned()
                .unwrap_or(FileState::Absent);
            let target = desired
                .image()
                .get(&change.path)
                .cloned()
                .ok_or(CheckpointError::InvalidPath)?;
            if inspect::path(root, &change.path, cancellation)? != before {
                return Err(CheckpointError::StalePreview.into());
            }
            if let FileState::Present {
                digest,
                bytes,
                metadata,
            } = &target
            {
                let mut source = archive.open_blob(*digest)?;
                stage.prepare(
                    Slot::new(index, SlotKind::Desired),
                    &mut source,
                    *digest,
                    *bytes,
                    *metadata,
                    cancellation,
                )?;
            }
            steps.push(Step {
                path: change.path,
                before,
                desired: target,
                index,
            });
        }
        Ok(Self { steps })
    }
    pub(super) fn steps(&self) -> &[Step] {
        &self.steps
    }
    pub(super) fn apply(
        &self,
        root: &Root,
        stage: &Stage,
        index: PathCount,
        cancellation: &CancellationToken,
    ) -> Result<(), Error> {
        let step = self
            .steps
            .get(index.get())
            .ok_or(CheckpointError::InvalidPath)?;
        if inspect::path(root, &step.path, cancellation)? != step.before {
            return Err(CheckpointError::StalePreview.into());
        }
        if cancellation.is_cancelled() {
            return Err(CheckpointError::CaptureTimedOut.into());
        }
        match (&step.before, &step.desired) {
            (FileState::Absent, FileState::Present { .. }) => root.install_create(
                &step.path,
                stage,
                Slot::new(step.index, SlotKind::Desired),
                ParentPolicy::CreateMissing,
            )?,
            (FileState::Present { .. }, FileState::Present { .. }) => {
                root.exchange(&step.path, stage, Slot::new(step.index, SlotKind::Desired))?;
                if stage.inspect(Slot::new(step.index, SlotKind::Desired), cancellation)?
                    != step.before
                {
                    return Err(CheckpointError::CaptureUnstable.into());
                }
            }
            (FileState::Present { .. }, FileState::Absent) => {
                root.take(&step.path, stage, Slot::new(step.index, SlotKind::Deleted))?;
                if stage.inspect(Slot::new(step.index, SlotKind::Deleted), cancellation)?
                    != step.before
                {
                    return Err(CheckpointError::CaptureUnstable.into());
                }
            }
            (FileState::Absent, FileState::Absent) => {
                return Err(CheckpointError::StalePreview.into());
            }
        }
        if inspect::path(root, &step.path, cancellation)? != step.desired {
            return Err(CheckpointError::CaptureUnstable.into());
        }
        Ok(())
    }
    pub(super) fn undo(
        &self,
        root: &Root,
        stage: &Stage,
        index: PathCount,
        cancellation: &CancellationToken,
    ) -> Result<Undo, Error> {
        let step = self
            .steps
            .get(index.get())
            .ok_or(CheckpointError::InvalidPath)?;
        if inspect::path(root, &step.path, cancellation)? != step.desired {
            return Ok(Undo::Conflict);
        }
        match (&step.before, &step.desired) {
            (FileState::Present { .. }, FileState::Present { .. }) => {
                let original = Slot::new(step.index, SlotKind::Desired);
                if stage.inspect(original, cancellation)? != step.before {
                    return Ok(Undo::Conflict);
                }
                root.exchange(&step.path, stage, original)?;
                // A same-user writer can race the precheck. Preserve the displaced
                // bytes and refuse a truthful rollback result if that happened.
                if stage.inspect(original, cancellation)? != step.desired {
                    return Ok(Undo::Conflict);
                }
            }
            (FileState::Present { .. }, FileState::Absent) => {
                let original = Slot::new(step.index, SlotKind::Deleted);
                if stage.inspect(original, cancellation)? != step.before {
                    return Ok(Undo::Conflict);
                }
                root.install_create(&step.path, stage, original, ParentPolicy::CreateMissing)?;
            }
            (FileState::Absent, FileState::Present { .. }) => {
                let removed = Slot::new(step.index, SlotKind::Deleted);
                root.take(&step.path, stage, removed)?;
                if stage.inspect(removed, cancellation)? != step.desired {
                    return Ok(Undo::Conflict);
                }
            }
            (FileState::Absent, FileState::Absent) => {
                return Err(CheckpointError::StalePreview.into());
            }
        }
        if inspect::path(root, &step.path, cancellation)? != step.before {
            return Ok(Undo::Conflict);
        }
        Ok(Undo::Restored)
    }
}
