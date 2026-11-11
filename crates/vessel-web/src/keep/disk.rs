//! Reading and writing the page's files safely: a bounded read of a
//! regular file, and an owner-only write that replaces the name in one
//! rename. Blocking: callers run these on `spawn_blocking`.
//!
//! [Decision Log]
//! - Purpose: the files hold what a person typed (drafts) and chose, in
//!   directories other programs (a dotfiles manager, a sync client) also
//!   write. A read must not hang on a FIFO, follow a name into a device or
//!   read a gigabyte; a write must not leave half a file, write through a
//!   link someone put at the name, or be readable by other users.
//! - Reading: the path's metadata must say a regular file, the open
//!   handle's must agree (the name could change between the two), and at
//!   most the bound plus one byte is read, so a file that grew meanwhile is
//!   still caught. A link to a regular file is read: a person may keep the
//!   file elsewhere and link it.
//! - Writing: a temporary file in the same directory, created new with
//!   mode `0600` on Unix, written, flushed to disk and renamed over the
//!   name. A rename replaces whatever the name held, a link included,
//!   rather than writing through it. A directory this creates is `0700`
//!   (the last component; parents are created as the system makes them).
//!   On Windows a new file takes its directory's permissions (the user's
//!   profile), and the rest is the same.
//! - Alternatives: the book's writer (`vessel-client`'s `BookFile`) writes
//!   with the default mode, and is another crate's; `O_NOFOLLOW` and
//!   `O_NONBLOCK` on open would need a dependency for their flags. What
//!   they would add over the two checks is a FIFO swapped in between the
//!   metadata and the open, by someone who can already replace files in
//!   the person's own directory.
//! - Not done: removing a temporary file a crash left between create and
//!   rename (another `vessel ui` may be writing it at that moment; it is a
//!   hidden file of at most the bound, named by its process).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};

/// Why a file was not read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadError {
    /// Something other than a regular file is at the name.
    #[error("not a regular file")]
    NotRegular,
    /// Larger than the bound.
    #[error("larger than {bound} bytes")]
    TooLarge {
        /// The bound.
        bound: u64,
    },
    /// The system refused.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// The file at `path`, at most `bound` bytes; `None` when there is none.
pub(crate) fn read(path: &Path, bound: u64) -> Result<Option<Vec<u8>>, ReadError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Err(ReadError::NotRegular);
    }
    if metadata.len() > bound {
        return Err(ReadError::TooLarge { bound });
    }
    let file = match File::open(path) {
        Ok(file) => file,
        // Removed between the two calls: as if it was never there.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !file.metadata()?.is_file() {
        return Err(ReadError::NotRegular);
    }
    let mut bytes = Vec::new();
    let _read = file.take(bound.saturating_add(1)).read_to_end(&mut bytes)?;
    if !u64::try_from(bytes.len()).is_ok_and(|read| read <= bound) {
        return Err(ReadError::TooLarge { bound });
    }
    Ok(Some(bytes))
}

/// Replace the file at `path` with `bytes`, owner-only, in one rename.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    ensure_directory(directory)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    let (temporary, file) = create_temporary(directory, &name.to_string_lossy())?;
    let written = fill(file, bytes).and_then(|()| fs::rename(&temporary, path));
    if written.is_err() {
        let _gone = fs::remove_file(&temporary);
    }
    written
}

/// Write `bytes` to `file` and flush them to the disk before it is
/// renamed, so a crash leaves the old file or the new one.
fn fill(mut file: File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()
}

/// `directory`, created owner-only when it does not exist; its parents as
/// the system makes them.
fn ensure_directory(directory: &Path) -> io::Result<()> {
    if directory.is_dir() {
        return Ok(());
    }
    if let Some(parent) = directory
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        let _builder = builder.mode(0o700);
    }
    match builder.create(directory) {
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && directory.is_dir() => Ok(()),
        created => created,
    }
}

/// A new owner-only file in `directory`, named after `name` and this
/// process, that nothing else uses.
fn create_temporary(directory: &Path, name: &str) -> io::Result<(PathBuf, File)> {
    let mut attempt = 0_u32;
    loop {
        let path = directory.join(format!(".{name}.{}.{attempt}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        let _options = options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            let _options = options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && attempt < 16 => {
                attempt = attempt.saturating_add(1);
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_file_reads_back_and_an_absent_one_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep").join("vessel").join("file.json");
        assert!(read(&path, 16).unwrap().is_none());
        write(&path, b"{\"version\":1}").unwrap();
        assert_eq!(read(&path, 16).unwrap().unwrap(), b"{\"version\":1}");
        write(&path, b"{}").unwrap();
        assert_eq!(read(&path, 16).unwrap().unwrap(), b"{}");
        // Nothing left beside it.
        let names: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["file.json"]);
    }

    #[test]
    fn an_oversized_file_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.json");
        fs::write(&path, [b' '; 17]).unwrap();
        assert!(matches!(
            read(&path, 16),
            Err(ReadError::TooLarge { bound: 16 })
        ));
        fs::write(&path, [b' '; 16]).unwrap();
        assert_eq!(read(&path, 16).unwrap().unwrap().len(), 16);
    }

    #[test]
    fn a_directory_at_the_name_is_not_read_nor_written_through() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.json");
        fs::create_dir(&path).unwrap();
        assert!(matches!(read(&path, 16), Err(ReadError::NotRegular)));
        assert!(write(&path, b"{}").is_err());
        assert!(path.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn written_owner_only_in_an_owner_only_directory() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("parent").join("vessel").join("file.json");
        write(&path, b"{}").unwrap();
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
        // A directory that was there keeps its mode.
        let open = dir.path().join("open");
        fs::create_dir(&open).unwrap();
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
        write(&open.join("file.json"), b"{}").unwrap();
        assert_eq!(mode(&open), 0o755);
        assert_eq!(mode(&open.join("file.json")), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_at_the_name_is_replaced_and_its_target_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("elsewhere.json");
        fs::write(&target, b"theirs").unwrap();
        let path = dir.path().join("file.json");
        std::os::unix::fs::symlink(&target, &path).unwrap();
        // Read through the link, as a person who keeps the file elsewhere
        // means.
        assert_eq!(read(&path, 16).unwrap().unwrap(), b"theirs");
        write(&path, b"ours").unwrap();
        assert!(
            !fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&path).unwrap(), b"ours");
        assert_eq!(fs::read(&target).unwrap(), b"theirs");
    }

    #[cfg(unix)]
    #[test]
    fn a_fifo_at_the_name_is_not_opened() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.json");
        let made = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(made.success());
        // Opening a FIFO for reading would wait for a writer forever; the
        // metadata check refuses it first.
        assert!(matches!(read(&path, 16), Err(ReadError::NotRegular)));
    }
}
