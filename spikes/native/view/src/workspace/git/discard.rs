//! The destructive decision shown before a file is restored or deleted.

use vessel_proto::workspace::{Discard, FileState, PathSet, RelPath, StatusEntry, WorkspaceRoot};

/// What discarding this file does, for the confirm dialog's explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardEffect {
    /// Restore HEAD into index and worktree, losing changes on both sides.
    Restore,
    /// An untracked file is removed by git clean.
    DeleteUntracked,
    /// An added file is absent from HEAD and is removed by restoration.
    DeleteAdded,
}

/// A decision tied to the root and file the person was shown. Construct
/// it before opening the kit confirm, consume it only on confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardDecision {
    root: WorkspaceRoot,
    path: RelPath,
    effect: DiscardEffect,
}

impl DiscardDecision {
    /// Prepare a decision from a status row; the node checks again when
    /// it acts. A rename is restored by its destination, as on the page.
    #[must_use]
    pub fn of(root: WorkspaceRoot, file: &StatusEntry) -> Self {
        let effect = if file.worktree == FileState::Untracked {
            DiscardEffect::DeleteUntracked
        } else if file.index == FileState::Added {
            DiscardEffect::DeleteAdded
        } else {
            DiscardEffect::Restore
        };
        Self {
            root,
            path: file.path.clone(),
            effect,
        }
    }

    /// The path the confirmation names.
    #[must_use]
    pub const fn path(&self) -> &RelPath {
        &self.path
    }

    /// What to explain before the confirm button can send the request.
    #[must_use]
    pub const fn effect(&self) -> DiscardEffect {
        self.effect
    }

    /// The person confirmed this decision; consume the exact root/path
    /// rather than consulting whichever session is open by then.
    #[must_use]
    pub fn confirm(self) -> Discard {
        Discard {
            root: self.root,
            paths: PathSet::Paths {
                paths: vec![self.path],
            },
            untracked: self.effect == DiscardEffect::DeleteUntracked,
        }
    }
}
