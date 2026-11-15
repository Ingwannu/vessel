//! Directory-relative managed-file reads and writes reuse fleet's safe handles.

use std::io::{Read as _, Write as _};

use sha2::{Digest as _, Sha256};
use vessel_proto::{BlobHash, BundlePath};

use super::mode::SavedMode;
use crate::dir::{Dir, Entry};
use crate::files::Mode;
use crate::orchestration::SkillError;
use crate::orchestration::bundle::MAX_SKILL_FILE_BYTES;

#[derive(Debug)]
pub(super) struct FileState {
    pub(super) bytes: Vec<u8>,
    pub(super) mode: SavedMode,
}

/// All traversals start at a held skill directory and reject links below it.
fn parent(root: &Dir, path: &BundlePath, create: Creation) -> Result<Option<Dir>, SkillError> {
    let mut parts = path.segments().peekable();
    let mut current = root.open_dir(".")?;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            return Ok(Some(current));
        }
        current = match current.entry(part)? {
            Entry::Dir => current.open_dir(part)?,
            Entry::Missing => match create {
                Creation::Read => return Ok(None),
                Creation::Write => current.create_dir(part)?,
            },
            Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
        };
    }
    Err(SkillError::Blocked)
}

#[derive(Debug, Clone, Copy)]
enum Creation {
    Read,
    Write,
}

fn leaf(path: &BundlePath) -> Result<&str, SkillError> {
    path.segments().last().ok_or(SkillError::Blocked)
}

pub(super) fn read(root: &Dir, path: &BundlePath) -> Result<Option<Vec<u8>>, SkillError> {
    Ok(snapshot(root, path)?.map(|state| state.bytes))
}

pub(super) fn snapshot(root: &Dir, path: &BundlePath) -> Result<Option<FileState>, SkillError> {
    let Some(parent) = parent(root, path, Creation::Read)? else {
        return Ok(None);
    };
    let leaf = leaf(path)?;
    match parent.entry(leaf)? {
        Entry::Missing => return Ok(None),
        Entry::File => {}
        Entry::Dir | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    }
    let mut bytes = Vec::new();
    let max = u64::try_from(MAX_SKILL_FILE_BYTES).map_err(|_bound| SkillError::Budget)?;
    let opened = parent.open_file(leaf)?;
    let mode = SavedMode::read(&opened.file)?;
    let _read = opened
        .file
        .take(max.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SKILL_FILE_BYTES {
        return Err(SkillError::Budget);
    }
    Ok(Some(FileState { bytes, mode }))
}

pub(super) fn write(root: &Dir, path: &BundlePath, bytes: &[u8]) -> Result<(), SkillError> {
    if bytes.len() > MAX_SKILL_FILE_BYTES {
        return Err(SkillError::Budget);
    }
    let parent = parent(root, path, Creation::Write)?.ok_or(SkillError::Blocked)?;
    let leaf = leaf(path)?;
    match parent.entry(leaf)? {
        Entry::Missing | Entry::File => {}
        Entry::Dir | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    }
    parent.replace::<SkillError>(leaf, Mode::Private, |file| Ok(file.write_all(bytes)?))
}

/// Rollback writes stay atomic and restore only the recorded ordinary mode bits.
pub(super) fn restore(root: &Dir, path: &BundlePath, state: &FileState) -> Result<(), SkillError> {
    if state.bytes.len() > MAX_SKILL_FILE_BYTES {
        return Err(SkillError::Budget);
    }
    let parent = parent(root, path, Creation::Write)?.ok_or(SkillError::Blocked)?;
    let leaf = leaf(path)?;
    match parent.entry(leaf)? {
        Entry::Missing | Entry::File => {}
        Entry::Dir | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    }
    parent.replace::<SkillError>(leaf, Mode::Private, |file| {
        file.write_all(&state.bytes)?;
        state.mode.restore(file)
    })
}

pub(super) fn remove_unchanged(
    root: &Dir,
    path: &BundlePath,
    expected: BlobHash,
) -> Result<(), SkillError> {
    let Some(bytes) = read(root, path)? else {
        return Ok(());
    };
    if hash(&bytes) != expected {
        return Ok(());
    }
    let parent = parent(root, path, Creation::Read)?.ok_or(SkillError::Blocked)?;
    parent.remove_file(leaf(path)?)?;
    Ok(())
}

pub(super) fn hash(bytes: &[u8]) -> BlobHash {
    BlobHash::from_bytes(Sha256::digest(bytes).into())
}

/// Whole-directory deletion is only used after a fresh explicit confirmation.
const MAX_REMOVAL_DEPTH: usize = 32;

pub(super) fn remove_directory(parent: &Dir, name: &str) -> Result<(), SkillError> {
    remove_tree(parent, name, 0)
}

fn remove_tree(parent: &Dir, name: &str, depth: usize) -> Result<(), SkillError> {
    if depth > MAX_REMOVAL_DEPTH {
        return Err(SkillError::Budget);
    }
    match parent.entry(name)? {
        Entry::Missing => return Ok(()),
        Entry::Dir => {}
        Entry::File | Entry::Symlink | Entry::Other => return Err(SkillError::Blocked),
    }
    let dir = parent.open_dir(name)?;
    dir.each_name(|child| {
        match dir.entry(child)? {
            Entry::Dir => {
                remove_tree(&dir, child, depth.saturating_add(1)).map_err(std::io::Error::other)?
            }
            Entry::File | Entry::Symlink => dir.remove_file(child)?,
            Entry::Missing => {}
            Entry::Other => return Err(std::io::Error::other("special file blocks skill removal")),
        }
        Ok(())
    })?;
    parent.remove_dir(name)?;
    Ok(())
}
