//! Cleanup walks only our typed private layout, never project paths or symlinks.
//!
//! [Decision Log]
//! - Purpose: retire private files without releasing quota before durable absence.
//! - Constraint: interrupted unlink/fsync is not an atomic database operation.
//! - Choice: bounded, repeatable FD-relative passes; only Complete acknowledges
//!   synced absence. Unknown names/links/mounts refuse and leave accounting held.
//! - Cost: a large archive takes several passes and failures retain its charge.
use super::{Cleanup, Directory, Error, Storage, name};
use crate::checkpoint::{account, rooted::MountId};
use rustix::fs::{AtFlags, Dir, Mode, OFlags};
use std::{ffi::CStr, fs::File, os::unix::fs::MetadataExt as _};
use vessel_proto::{
    BlobHash, WorktreeId,
    checkpoint::{
        CHECKPOINT_PAGE_ENTRIES, CaptureId, CheckpointError, PathCount, RestoreId, RestorePlanId,
    },
};

const CLEANUP_ENTRIES: usize = CHECKPOINT_PAGE_ENTRIES;
const REGULAR: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK)
    .union(OFlags::CLOEXEC);
#[derive(Clone, Copy)]
enum Layout {
    Capture,
    Blobs,
    Previews,
    Restores,
    Restore,
    Stage,
}
#[derive(Clone, Copy)]
enum Entry {
    Folder(Layout),
    File,
}
struct Budget {
    remaining: usize,
}
impl Storage {
    /// Remove one retired capture in bounded passes after all workers and pins end.
    /// The caller retains its worker permit and durable garbage charge through I/O.
    /// Complete means the exact namespace is absent and its parent was synced.
    /// # Errors
    /// Unknown names, unsafe metadata/mounts, or an actual I/O/fsync failure.
    pub fn clean_capture(
        &self,
        worktree: WorktreeId,
        capture: CaptureId,
    ) -> Result<Cleanup, Error> {
        let worktree = match self.folder.child(&name(worktree)?) {
            Ok(folder) => folder,
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                self.folder.file.sync_all()?;
                return Ok(Cleanup::Complete);
            }
            Err(error) => return Err(error),
        };
        let capture = name(capture)?;
        let folder = match worktree.child(&capture) {
            Ok(folder) => folder,
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                worktree.file.sync_all()?;
                return Ok(Cleanup::Complete);
            }
            Err(error) => return Err(error),
        };
        let mut budget = Budget {
            remaining: CLEANUP_ENTRIES,
        };
        if walk(&folder, Layout::Capture, &mut budget)? == Cleanup::Pending {
            return Ok(Cleanup::Pending);
        }
        remove_folder(&worktree, &capture, &folder)?;
        // A same-user external writer can race us; never acknowledge a recreated
        // namespace. All cooperating node jobs are excluded by caller admission.
        match worktree.child(&capture) {
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Cleanup::Complete)
            }
            Ok(_) => Err(CheckpointError::CaptureUnstable.into()),
            Err(error) => Err(error),
        }
    }
}
fn canonical<T: std::str::FromStr + std::fmt::Display>(text: &str) -> bool {
    text.parse::<T>()
        .is_ok_and(|value| value.to_string() == text)
}
fn classify(layout: Layout, name: &CStr) -> Result<Entry, Error> {
    let text = name
        .to_str()
        .map_err(|_invalid| CheckpointError::UnsafeEntry)?;
    let entry = match layout {
        Layout::Capture => match text {
            "blobs" => Entry::Folder(Layout::Blobs),
            "previews" => Entry::Folder(Layout::Previews),
            "restores" => Entry::Folder(Layout::Restores),
            _ => return Err(CheckpointError::UnsafeEntry.into()),
        },
        Layout::Blobs if text == "candidate" || canonical::<BlobHash>(text) => Entry::File,
        Layout::Previews if canonical::<RestorePlanId>(text) => Entry::Folder(Layout::Blobs),
        Layout::Restores if canonical::<RestoreId>(text) => Entry::Folder(Layout::Restore),
        Layout::Restore if text == "stage" => Entry::Folder(Layout::Stage),
        Layout::Stage if stage_slot(text) => Entry::File,
        Layout::Blobs | Layout::Previews | Layout::Restores | Layout::Restore | Layout::Stage => {
            return Err(CheckpointError::UnsafeEntry.into());
        }
    };
    Ok(entry)
}
fn stage_slot(text: &str) -> bool {
    let Some(index) = text
        .strip_prefix("desired-")
        .or_else(|| text.strip_prefix("deleted-"))
    else {
        return false;
    };
    index
        .parse::<usize>()
        .ok()
        .is_some_and(|value| value.to_string() == index && PathCount::new(value).is_ok())
}
fn walk(folder: &Directory, layout: Layout, budget: &mut Budget) -> Result<Cleanup, Error> {
    account::private_directory(&folder.file)?;
    let mut entries = Dir::read_from(&folder.file).map_err(std::io::Error::from)?;
    for entry in &mut entries {
        let entry = entry.map_err(std::io::Error::from)?;
        let name = entry.file_name();
        if name == c"." || name == c".." {
            continue;
        }
        if budget.remaining == 0 {
            return Ok(Cleanup::Pending);
        }
        budget.remaining = budget.remaining.saturating_sub(1);
        match classify(layout, name)? {
            Entry::Folder(child_layout) => {
                let child = folder.child(name)?;
                if walk(&child, child_layout, budget)? == Cleanup::Pending {
                    return Ok(Cleanup::Pending);
                }
                remove_folder(folder, name, &child)?;
            }
            Entry::File => remove_file(folder, name)?,
        }
    }
    folder.file.sync_all()?;
    Ok(Cleanup::Complete)
}
fn same_named(parent: &Directory, name: &CStr, opened: &File) -> Result<(), Error> {
    let named = rustix::fs::statat(&parent.file, name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(std::io::Error::from)?;
    let actual = opened.metadata()?;
    if named.st_dev != actual.dev() || named.st_ino != actual.ino() {
        return Err(CheckpointError::CaptureUnstable.into());
    }
    Ok(())
}
fn remove_folder(parent: &Directory, name: &CStr, child: &Directory) -> Result<(), Error> {
    account::private_directory(&parent.file)?;
    account::private_directory(&child.file)?;
    same_named(parent, name, &child.file)?;
    rustix::fs::unlinkat(&parent.file, name, AtFlags::REMOVEDIR).map_err(std::io::Error::from)?;
    parent.file.sync_all()?;
    Ok(())
}
fn remove_file(parent: &Directory, name: &CStr) -> Result<(), Error> {
    let file = File::from(
        rustix::fs::openat(&parent.file, name, REGULAR, Mode::empty())
            .map_err(std::io::Error::from)?,
    );
    let metadata = file.metadata()?;
    account::owned(&metadata)?;
    if !metadata.is_file() || metadata.nlink() != 1 || MountId::of(&file)? != parent.mount {
        return Err(CheckpointError::UnsafeEntry.into());
    }
    crate::checkpoint::rooted::refuse_attributes(&file)?;
    same_named(parent, name, &file)?;
    rustix::fs::unlinkat(&parent.file, name, AtFlags::empty()).map_err(std::io::Error::from)?;
    parent.file.sync_all()?;
    Ok(())
}

#[cfg(test)]
pub(super) const fn test_pass_limit() -> usize {
    CLEANUP_ENTRIES
}
