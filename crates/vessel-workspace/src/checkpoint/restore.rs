//! Per-path apply/undo only. The node owns durable journal ordering and final status.
//!
//! All desired regular files are staged before the first project mutation.
//! Existing replacements exchange inodes, retaining the original inside the
//! journal. Rollback refuses an external change rather than overwriting it.
mod journal;
mod original;
pub(super) mod stage;
#[cfg(test)]
mod tests;

use super::{
    Error,
    archive::Archive,
    inspect,
    rooted::{Identity, Root, mutation::ParentPolicy},
};
use original::Original;
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
    before_identity: Original,
    desired_identity: Option<vessel_proto::checkpoint::ObjectFingerprint>,
    index: PathCount,
}
#[derive(Debug)]
pub(super) struct Prepared {
    steps: Vec<Step>,
    mode: Mode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Fresh,
    Recovery,
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
        root.check_mount(&stage.folder)?;
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
            let before_identity = root.entry_identity(&change.path)?;
            if inspect::path(root, &change.path, cancellation)? != before
                || root.entry_identity(&change.path)? != before_identity
            {
                return Err(CheckpointError::StalePreview.into());
            }
            let desired_identity = if let FileState::Present {
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
                Some(original::object(
                    stage.identity(Slot::new(index, SlotKind::Desired))?,
                ))
            } else {
                None
            };
            steps.push(Step {
                path: change.path,
                before,
                desired: target,
                before_identity: Original::from(before_identity),
                desired_identity,
                index,
            });
        }
        Ok(Self {
            steps,
            mode: Mode::Fresh,
        })
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
        if self.mode != Mode::Fresh
            || !step
                .before_identity
                .fresh_matches(root.entry_identity(&step.path)?)
            || inspect::path(root, &step.path, cancellation)? != step.before
            || !step
                .before_identity
                .fresh_matches(root.entry_identity(&step.path)?)
        {
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
                    || !same_entry(
                        Some(stage.identity(Slot::new(step.index, SlotKind::Desired))?),
                        step.before_identity.object(),
                    )
                {
                    return Err(CheckpointError::CaptureUnstable.into());
                }
            }
            (FileState::Present { .. }, FileState::Absent) => {
                root.take(&step.path, stage, Slot::new(step.index, SlotKind::Deleted))?;
                if stage.inspect(Slot::new(step.index, SlotKind::Deleted), cancellation)?
                    != step.before
                    || !same_entry(
                        Some(stage.identity(Slot::new(step.index, SlotKind::Deleted))?),
                        step.before_identity.object(),
                    )
                {
                    return Err(CheckpointError::CaptureUnstable.into());
                }
            }
            (FileState::Absent, FileState::Absent) => {
                return Err(CheckpointError::StalePreview.into());
            }
        }
        if inspect::path(root, &step.path, cancellation)? != step.desired
            || !same_entry(root.entry_identity(&step.path)?, step.desired_identity)
        {
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
        if inspect::path(root, &step.path, cancellation)? != step.desired
            || !same_entry(root.entry_identity(&step.path)?, step.desired_identity)
        {
            return Ok(Undo::Conflict);
        }
        match (&step.before, &step.desired) {
            (FileState::Present { .. }, FileState::Present { .. }) => {
                let original = Slot::new(step.index, SlotKind::Desired);
                if stage.inspect(original, cancellation)? != step.before
                    || !same_entry(
                        Some(stage.identity(original)?),
                        step.before_identity.object(),
                    )
                {
                    return Ok(Undo::Conflict);
                }
                root.exchange(&step.path, stage, original)?;
                // A same-user writer can race the precheck. Preserve the displaced
                // bytes and refuse a truthful rollback result if that happened.
                if stage.inspect(original, cancellation)? != step.desired
                    || !same_entry(Some(stage.identity(original)?), step.desired_identity)
                {
                    return Ok(Undo::Conflict);
                }
            }
            (FileState::Present { .. }, FileState::Absent) => {
                let original = Slot::new(step.index, SlotKind::Deleted);
                if stage.inspect(original, cancellation)? != step.before
                    || !same_entry(
                        Some(stage.identity(original)?),
                        step.before_identity.object(),
                    )
                {
                    return Ok(Undo::Conflict);
                }
                root.install_create(&step.path, stage, original, ParentPolicy::CreateMissing)?;
            }
            (FileState::Absent, FileState::Present { .. }) => {
                let removed = Slot::new(step.index, SlotKind::Deleted);
                root.take(&step.path, stage, removed)?;
                if stage.inspect(removed, cancellation)? != step.desired
                    || !same_entry(Some(stage.identity(removed)?), step.desired_identity)
                {
                    return Ok(Undo::Conflict);
                }
            }
            (FileState::Absent, FileState::Absent) => {
                return Err(CheckpointError::StalePreview.into());
            }
        }
        if inspect::path(root, &step.path, cancellation)? != step.before
            || !same_entry(
                root.entry_identity(&step.path)?,
                step.before_identity.object(),
            )
        {
            return Ok(Undo::Conflict);
        }
        Ok(Undo::Restored)
    }
}
fn same_entry(
    current: Option<Identity>,
    expected: Option<vessel_proto::checkpoint::ObjectFingerprint>,
) -> bool {
    current.map(original::object) == expected
}
