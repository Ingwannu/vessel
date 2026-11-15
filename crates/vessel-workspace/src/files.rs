//! Files of a root (protocol 10): list a folder, read a page, write a text
//! file, create a folder, rename, delete. Every path goes through
//! [`Root`]'s checks first; every answer is bounded.
//!
//! [Decision Log]
//! - Writes are whole text files of at most 1 MiB (Paseo's and GOAT's
//!   bound for the editor), written to a temporary file beside the target
//!   and renamed over it, so a reader never sees half a file; the old
//!   file's permissions are kept. A write states what it expects to find
//!   ([`WriteExpect`]): nothing, or the SHA-256 of the content it read
//!   (files of at most 8 MiB carry one), never a time: a coarse clock
//!   (FAT's 2 s, HFS+'s 1 s) keeps size and time across an edit, and a
//!   page cannot hold a time in nanoseconds exactly (protocol 11, the
//!   `FileVersion` Decision Log in `vessel-proto`).
//! - Reads are pages of at most 1 MiB from an offset, never the whole file;
//!   binary files are recognised (a NUL byte in the first 8 KiB, or bytes
//!   that are not UTF-8) and not sent.
//! - Everything here blocks on the disk; each public call runs on the
//!   blocking pool.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};
use vessel_proto::workspace::{
    ContentHash, DirEntry, DirListing, EntryKind, FileRead, FileVersion, RelPath, WriteExpect,
};

use crate::paths::{PathError, Root, Target};

pub mod binary;
pub mod duplicate;
pub mod inspect;
pub mod observe;
pub mod suggestions;

/// The most entries a listing holds.
pub const MAX_ENTRIES: usize = 5000;
/// The largest page read.
pub const MAX_PAGE: u32 = 1024 * 1024;
/// A page's size when not asked.
pub const DEFAULT_PAGE: u32 = 256 * 1024;
/// The largest text written.
pub const MAX_WRITE: usize = 1024 * 1024;
/// Files up to this size get a content hash in their version.
pub const MAX_HASHED: u64 = 8 * 1024 * 1024;
/// How much of the start is read to tell text from binary.
const SNIFF: usize = 8 * 1024;

/// Why a file call failed.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    /// The path's checks (outside, `.git`, missing, the root).
    #[error(transparent)]
    Path(#[from] PathError),
    /// A folder where a file is wanted, or the other way round.
    #[error("`{0}` is not a {1}")]
    Kind(RelPath, &'static str),
    /// What a write expected is not what is there.
    #[error("`{0}` changed since it was read")]
    Changed(RelPath),
    /// Something is there already.
    #[error("`{0}` exists already")]
    Exists(RelPath),
    /// A folder that is not empty, without `recursive`.
    #[error("`{0}` is not empty")]
    NotEmpty(RelPath),
    /// Text longer than [`MAX_WRITE`].
    #[error("a file written here is at most {MAX_WRITE} bytes")]
    TooLarge,
    /// A binary transfer or duplicate exceeds its whole-source budget.
    #[error(
        "file data is at most {} bytes",
        vessel_proto::workspace::panel_files::MAX_FILE_BYTES
    )]
    TransferTooLarge,
    /// A byte position is beyond the source's end.
    #[error("the requested byte offset is beyond the file")]
    Offset,
    /// A copy cannot traverse links, special files, git metadata or an unbounded tree.
    #[error("copy refused: links, special files, git metadata or copy limits")]
    CopyLimit,
    /// An actual filesystem name cannot be represented by the portable path contract.
    #[error("a directory entry has an invalid portable path")]
    InvalidEntry(#[source] vessel_proto::workspace::InvalidRelPath),
    /// The disk refused.
    #[error("the disk refused")]
    Io(#[source] io::Error),
    /// The blocking task did not finish.
    #[error("the file task stopped")]
    Task,
}

/// Run `work` on the blocking pool.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, FileError> + Send + 'static,
) -> Result<T, FileError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_panicked| FileError::Task)?
}

/// A folder's entries (`.git` left out), folders first, then by name.
/// [`list_marked`] adds what git ignores.
///
/// # Errors
/// As [`FileError`] says.
pub async fn list(root: &Root, dir: &RelPath) -> Result<DirListing, FileError> {
    let (root, dir) = (root.clone(), dir.clone());
    blocking(move || {
        let folder = root.existing(&dir)?;
        if !folder.is_dir() {
            return Err(FileError::Kind(dir, "folder"));
        }
        let mut entries = Vec::new();
        let mut more = false;
        for entry in fs::read_dir(&folder).map_err(FileError::Io)? {
            let entry = entry.map_err(FileError::Io)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.eq_ignore_ascii_case(".git") {
                continue;
            }
            if entries.len() >= MAX_ENTRIES {
                more = true;
                break;
            }
            let kind = entry.file_type().map_or(EntryKind::Other, |kind| {
                if kind.is_symlink() {
                    EntryKind::Symlink
                } else if kind.is_dir() {
                    EntryKind::Dir
                } else if kind.is_file() {
                    EntryKind::File
                } else {
                    EntryKind::Other
                }
            });
            let size = (kind == EntryKind::File)
                .then(|| entry.metadata().ok().map(|metadata| metadata.len()))
                .flatten();
            entries.push(DirEntry {
                name,
                kind,
                size,
                ignored: false,
            });
        }
        entries.sort_by(|a, b| {
            (a.kind != EntryKind::Dir, a.name.as_str())
                .cmp(&(b.kind != EntryKind::Dir, b.name.as_str()))
        });
        Ok(DirListing { entries, more })
    })
    .await
}

/// [`list`], with the entries git ignores marked, and left out unless
/// `with_ignored`.
///
/// # Errors
/// As [`list`], and git's failures as [`FileError::Io`].
pub async fn list_marked(
    git: &crate::Git,
    root: &Root,
    dir: &RelPath,
    with_ignored: bool,
) -> Result<DirListing, FileError> {
    let mut listing = list(root, dir).await?;
    let paths: Vec<RelPath> = listing
        .entries
        .iter()
        .filter_map(|entry| dir.child(&entry.name).ok())
        .collect();
    let ignored = git
        .ignored(root.path(), &paths)
        .await
        .map_err(|error| FileError::Io(io::Error::other(error.to_string())))?;
    for entry in &mut listing.entries {
        entry.ignored = dir
            .child(&entry.name)
            .is_ok_and(|path| ignored.contains(&path));
    }
    if !with_ignored {
        listing.entries.retain(|entry| !entry.ignored);
    }
    Ok(listing)
}

/// A page of `path` from `offset`, at most `limit` bytes (default
/// [`DEFAULT_PAGE`], at most [`MAX_PAGE`]).
///
/// # Errors
/// As [`FileError`] says.
pub async fn read(
    root: &Root,
    path: &RelPath,
    offset: u64,
    limit: Option<u32>,
) -> Result<FileRead, FileError> {
    let (root, path) = (root.clone(), path.clone());
    let limit = limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    blocking(move || {
        let file = root.existing(&path)?;
        let mut handle = open_file(&file, &path)?;
        let version = version_of(&mut handle)?;
        let mut start = vec![0_u8; SNIFF];
        let sniffed = read_up_to(&mut handle, &mut start)?;
        start.truncate(sniffed);
        let _at = handle
            .seek(SeekFrom::Start(offset))
            .map_err(FileError::Io)?;
        let mut page = vec![0_u8; usize::try_from(limit).unwrap_or(usize::MAX)];
        let read = read_up_to(&mut handle, &mut page)?;
        page.truncate(read);
        let content = crate::text::page(&start, &page, offset, version.size);
        Ok(FileRead { version, content })
    })
    .await
}

/// The largest blob `git.show` pages (a larger one is refused).
pub const MAX_BLOB: usize = 16 * 1024 * 1024;

/// A page of a blob already read (`git.show`), as [`read`] pages a file:
/// its version is its size and hash (a blob has no change time).
#[must_use]
pub fn blob_page(bytes: &[u8], offset: u64, limit: Option<u32>) -> FileRead {
    let limit = usize::try_from(limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE)).unwrap_or(1);
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let start = bytes.get(..SNIFF.min(bytes.len())).unwrap_or_default();
    let from = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(bytes.len());
    let page = bytes
        .get(from..from.saturating_add(limit).min(bytes.len()))
        .unwrap_or_default();
    let hash = ContentHash::from_sha256(Sha256::digest(bytes).into());
    FileRead {
        version: FileVersion {
            size,
            modified: 0,
            hash: Some(hash),
        },
        content: crate::text::page(start, page, offset, size),
    }
}

/// Read until `buffer` is full or the file ends; how many bytes were read.
fn read_up_to(file: &mut File, buffer: &mut [u8]) -> Result<usize, FileError> {
    let mut filled = 0;
    while let Some(rest) = buffer.get_mut(filled..) {
        if rest.is_empty() {
            break;
        }
        match file.read(rest) {
            Ok(0) => break,
            Ok(read) => filled = filled.saturating_add(read),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(FileError::Io(error)),
        }
    }
    Ok(filled)
}

/// `file` (at `path`) opened, when it is a regular file (`regular`).
fn open_file(file: &Path, path: &RelPath) -> Result<File, FileError> {
    crate::regular::open(file)
        .map_err(FileError::Io)?
        .ok_or_else(|| FileError::Kind(path.clone(), "file"))
}

/// An open file's version: size, change time (milliseconds), and its
/// content's hash when small; read from its start, and left there.
fn version_of(file: &mut File) -> Result<FileVersion, FileError> {
    let metadata = file.metadata().map_err(FileError::Io)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|since| u64::try_from(since.as_millis()).ok())
        .unwrap_or(0);
    let hash = if metadata.len() <= MAX_HASHED {
        let mut hasher = Sha256::new();
        let mut chunk = vec![0_u8; 64 * 1024];
        loop {
            let read = read_up_to(file, &mut chunk)?;
            hasher.update(chunk.get(..read).unwrap_or_default());
            if read < chunk.len() {
                break;
            }
        }
        let _start = file.seek(SeekFrom::Start(0)).map_err(FileError::Io)?;
        Some(ContentHash::from_sha256(hasher.finalize().into()))
    } else {
        None
    };
    Ok(FileVersion {
        size: metadata.len(),
        modified,
        hash,
    })
}

/// Write `text` to `path` as `expect` allows; the new version.
///
/// # Errors
/// As [`FileError`] says.
pub async fn write(
    root: &Root,
    path: &RelPath,
    text: String,
    expect: WriteExpect,
) -> Result<FileVersion, FileError> {
    if text.len() > MAX_WRITE {
        return Err(FileError::TooLarge);
    }
    let (root, path) = (root.clone(), path.clone());
    blocking(move || {
        let Target {
            path: target,
            exists,
            ..
        } = root.write_target(&path)?;
        if exists && !target.is_file() {
            return Err(FileError::Kind(path, "file"));
        }
        match (&expect, exists) {
            (WriteExpect::Absent, true) => return Err(FileError::Exists(path)),
            (WriteExpect::Content { .. }, false) => return Err(FileError::Changed(path)),
            (WriteExpect::Content { hash }, true) => {
                // A file past the hashed bound has no hash, so it is not the
                // content the caller read (which had one).
                let now = version_of(&mut open_file(&target, &path)?)?;
                if now.hash.as_ref() != Some(hash) {
                    return Err(FileError::Changed(path));
                }
            }
            (WriteExpect::Absent, false) | (WriteExpect::Any, _) => {}
        }
        replace(&target, text.as_bytes(), exists)?;
        version_of(&mut open_file(&target, &path)?)
    })
    .await
}

/// Write `bytes` beside `target` and rename it over, keeping the old
/// file's permissions.
fn replace(target: &Path, bytes: &[u8], exists: bool) -> Result<(), FileError> {
    let folder = target.parent().ok_or(FileError::Task)?;
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = unique(folder, &name);
    let written = (|| {
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if exists {
            let permissions = fs::metadata(target)?.permissions();
            fs::set_permissions(&temporary, permissions)?;
        }
        fs::rename(&temporary, target)
    })();
    if let Err(error) = written {
        let _removed = fs::remove_file(&temporary);
        return Err(FileError::Io(error));
    }
    Ok(())
}

/// A name beside `name` in `folder` that nothing uses.
fn unique(folder: &Path, name: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    folder.join(format!(".{name}.vessel-{}-{stamp:08x}", std::process::id()))
}

/// Create the folder `path` (its parent must exist).
///
/// # Errors
/// As [`FileError`] says.
pub async fn create_dir(root: &Root, path: &RelPath) -> Result<(), FileError> {
    let (root, path) = (root.clone(), path.clone());
    blocking(move || {
        let target = root.entry(&path)?;
        if target.exists {
            return Err(FileError::Exists(path));
        }
        fs::create_dir(&target.path).map_err(FileError::Io)
    })
    .await
}

/// Move `from` to `to` within the root; never over something.
///
/// # Errors
/// As [`FileError`] says.
pub async fn rename(root: &Root, from: &RelPath, to: &RelPath) -> Result<(), FileError> {
    let (root, from, to) = (root.clone(), from.clone(), to.clone());
    blocking(move || {
        let source = root.entry(&from)?;
        if !source.exists {
            return Err(FileError::Path(PathError::NotFound(from)));
        }
        let target = root.entry(&to)?;
        if target.exists {
            return Err(FileError::Exists(to));
        }
        if to.starts_with(&from) {
            return Err(FileError::Kind(to, "place outside what is moved"));
        }
        fs::rename(&source.path, &target.path).map_err(FileError::Io)
    })
    .await
}

/// Delete `path`: a file or a link (never what it points to), or a folder
/// (only an empty one unless `recursive`). Never the root.
///
/// # Errors
/// As [`FileError`] says.
pub async fn delete(root: &Root, path: &RelPath, recursive: bool) -> Result<(), FileError> {
    let (root, path) = (root.clone(), path.clone());
    blocking(move || {
        let target = root.entry(&path)?;
        if !target.exists {
            return Err(FileError::Path(PathError::NotFound(path)));
        }
        let metadata = fs::symlink_metadata(&target.path).map_err(FileError::Io)?;
        if !metadata.is_dir() {
            return fs::remove_file(&target.path).map_err(FileError::Io);
        }
        if recursive {
            // `remove_dir_all` does not follow links inside.
            return fs::remove_dir_all(&target.path).map_err(FileError::Io);
        }
        fs::remove_dir(&target.path).map_err(|error| {
            if fs::read_dir(&target.path).is_ok_and(|mut entries| entries.next().is_some()) {
                FileError::NotEmpty(path)
            } else {
                FileError::Io(error)
            }
        })
    })
    .await
}

#[cfg(test)]
mod tests;
