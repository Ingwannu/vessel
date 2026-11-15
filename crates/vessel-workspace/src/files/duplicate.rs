//! Exclusive sibling copies. The destination is reserved before any
//! content is copied; failure removes only the destination we created.
//! Directory copies refuse links/special files instead of importing data
//! from a different root or blocking on a FIFO.

use std::fs::{self, File, FileTimes, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::path::Path;

use vessel_proto::workspace::RelPath;
use vessel_proto::workspace::panel_files::{MAX_FILE_BYTES, MAX_PAGE_BYTES};

use super::{FileError, blocking};
use crate::paths::Root;

/// Limits bound recursive filesystem work independently of file bytes.
pub const MAX_COPY_ENTRIES: usize = 20_000;
/// Deep generated trees must not exhaust the blocking thread's stack.
pub const MAX_COPY_DEPTH: usize = 64;
/// A bounded name search prevents a hostile directory consuming a worker.
pub const MAX_COPY_NAMES: usize = 10_000;

/// Copy to `name copy.ext`, then `name copy 2.ext`, without overwriting.
///
/// # Errors
/// Refuses roots, symlinks, special files, oversized trees and disk failures.
pub async fn duplicate(root: &Root, path: &RelPath) -> Result<RelPath, FileError> {
    let (root, path) = (root.clone(), path.clone());
    blocking(move || copy_sibling(&root, &path)).await
}

fn copy_sibling(root: &Root, path: &RelPath) -> Result<RelPath, FileError> {
    let source = root.entry(path)?;
    let metadata = fs::symlink_metadata(&source.path).map_err(FileError::Io)?;
    if metadata.file_type().is_symlink() || !(metadata.is_dir() || metadata.is_file()) {
        return Err(FileError::Kind(path.clone(), "regular file or directory"));
    }
    let parent = path.parent().ok_or(crate::paths::PathError::Root)?;
    let name = path.file_name().ok_or(crate::paths::PathError::Root)?;
    for ordinal in 1..=MAX_COPY_NAMES {
        let name = copy_name(name, metadata.is_dir(), ordinal);
        let candidate = parent
            .child(&name)
            .map_err(|_invalid| FileError::CopyLimit)?;
        let target = root.entry(&candidate)?;
        let reserved = reserve(&target.path, metadata.is_dir());
        match reserved {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(FileError::Io(error)),
            Ok(file) => {
                let mut budget = Budget::default();
                let result = copy_reserved(&source.path, &target.path, file, &mut budget, 0);
                if let Err(error) = result {
                    // Cleanup failure is itself material: report the disk
                    // error rather than claiming the incomplete copy is gone.
                    remove_created(&target.path, metadata.is_dir()).map_err(FileError::Io)?;
                    return Err(error);
                }
                return Ok(candidate);
            }
        }
    }
    Err(FileError::CopyLimit)
}

fn copy_name(name: &str, directory: bool, ordinal: usize) -> String {
    let (stem, extension) = if directory {
        (name, None)
    } else {
        name.rsplit_once('.')
            .filter(|(stem, _)| !stem.is_empty())
            .map_or((name, None), |(stem, extension)| (stem, Some(extension)))
    };
    let suffix = if ordinal == 1 {
        " copy".to_owned()
    } else {
        format!(" copy {ordinal}")
    };
    extension.map_or_else(
        || format!("{stem}{suffix}"),
        |ext| format!("{stem}{suffix}.{ext}"),
    )
}

fn reserve(path: &Path, directory: bool) -> io::Result<Option<File>> {
    if directory {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            let _private = builder.mode(0o700);
        }
        builder.create(path)?;
        Ok(None)
    } else {
        let mut options = OpenOptions::new();
        let _exclusive = options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            let _private = options.mode(0o600);
        }
        options.open(path).map(Some)
    }
}

#[derive(Default)]
struct Budget {
    entries: usize,
    bytes: u64,
}

fn copy_reserved(
    source: &Path,
    target: &Path,
    reserved: Option<File>,
    budget: &mut Budget,
    depth: usize,
) -> Result<(), FileError> {
    budget.entries = budget.entries.saturating_add(1);
    if depth > MAX_COPY_DEPTH || budget.entries > MAX_COPY_ENTRIES {
        return Err(FileError::CopyLimit);
    }
    let metadata = fs::symlink_metadata(source).map_err(FileError::Io)?;
    if metadata.file_type().is_symlink() {
        return Err(FileError::CopyLimit);
    }
    if metadata.is_dir() {
        for child in fs::read_dir(source).map_err(FileError::Io)? {
            let child = child.map_err(FileError::Io)?;
            let name = child.file_name();
            if name
                .to_str()
                .is_none_or(|name| name.eq_ignore_ascii_case(".git"))
            {
                return Err(FileError::CopyLimit);
            }
            let kind = child.file_type().map_err(FileError::Io)?;
            if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
                return Err(FileError::CopyLimit);
            }
            let destination = target.join(name);
            let file = reserve(&destination, kind.is_dir()).map_err(FileError::Io)?;
            copy_reserved(
                &child.path(),
                &destination,
                file,
                budget,
                depth.saturating_add(1),
            )?;
        }
    } else {
        let mut input = crate::regular::open(source)
            .map_err(FileError::Io)?
            .ok_or(FileError::CopyLimit)?;
        let mut output = reserved.ok_or(FileError::CopyLimit)?;
        let mut buffer = vec![0; MAX_PAGE_BYTES];
        loop {
            let read = input.read(&mut buffer).map_err(FileError::Io)?;
            if read == 0 {
                break;
            }
            budget.bytes = budget
                .bytes
                .saturating_add(u64::try_from(read).map_err(|_invalid| FileError::Task)?);
            if budget.bytes > MAX_FILE_BYTES {
                return Err(FileError::TransferTooLarge);
            }
            output
                .write_all(buffer.get(..read).ok_or(FileError::Task)?)
                .map_err(FileError::Io)?;
        }
        output.sync_all().map_err(FileError::Io)?;
        let mut times = FileTimes::new();
        if let Ok(modified) = metadata.modified() {
            times = times.set_modified(modified);
        }
        if let Ok(accessed) = metadata.accessed() {
            times = times.set_accessed(accessed);
        }
        output.set_times(times).map_err(FileError::Io)?;
    }
    fs::set_permissions(target, metadata.permissions()).map_err(FileError::Io)
}

fn remove_created(path: &Path, directory: bool) -> io::Result<()> {
    if directory {
        writable_directories(path)?;
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

// Completed child directories may already have the source's readonly
// mode. Only our new copy is made removable, never the source tree.
fn writable_directories(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = metadata.permissions().mode() | 0o700;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    for child in fs::read_dir(path)? {
        writable_directories(&child?.path())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn copies_exact_bytes_and_uses_first_exclusive_available_name() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(temp.path()).unwrap();
        let source: RelPath = "a.tar.gz".parse().unwrap();
        fs::write(temp.path().join("a.tar.gz"), b"\0\xfftest").unwrap();
        assert_eq!(
            duplicate(&root, &source).await.unwrap().as_str(),
            "a.tar copy.gz"
        );
        assert_eq!(
            duplicate(&root, &source).await.unwrap().as_str(),
            "a.tar copy 2.gz"
        );
        assert_eq!(
            fs::read(temp.path().join("a.tar copy.gz")).unwrap(),
            b"\0\xfftest"
        );
        assert!(duplicate(&root, &RelPath::root()).await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nested_external_links_refuse_and_remove_the_partial_copy() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("dir")).unwrap();
        fs::write(temp.path().join("dir/a"), "safe").unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("dir/link")).unwrap();
        let root = Root::open(temp.path()).unwrap();
        assert!(duplicate(&root, &"dir".parse().unwrap()).await.is_err());
        assert!(!temp.path().join("dir copy").exists());
    }

    #[test]
    fn dotfiles_directories_and_extensions_keep_the_reference_naming() {
        assert_eq!(copy_name(".env", false, 1), ".env copy");
        assert_eq!(copy_name("dir.ext", true, 2), "dir.ext copy 2");
        assert_eq!(copy_name("main.rs", false, 1), "main copy.rs");
    }

    #[tokio::test]
    async fn concurrent_copies_never_overwrite_each_other() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("file"), "content").unwrap();
        let root = Root::open(temp.path()).unwrap();
        let path: RelPath = "file".parse().unwrap();
        let (first, second) = tokio::join!(duplicate(&root, &path), duplicate(&root, &path));
        let (first, second) = (first.unwrap(), second.unwrap());
        assert_ne!(first, second);
        assert_eq!(fs::read(first.under(temp.path())).unwrap(), b"content");
        assert_eq!(fs::read(second.under(temp.path())).unwrap(), b"content");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn copied_private_modes_remain_private_and_readonly_cleanup_preserves_source() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("private"), "content").unwrap();
        fs::set_permissions(
            temp.path().join("private"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let root = Root::open(temp.path()).unwrap();
        let copy = duplicate(&root, &"private".parse().unwrap()).await.unwrap();
        assert_eq!(
            fs::metadata(copy.under(temp.path()))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        fs::create_dir_all(temp.path().join("tree/readonly")).unwrap();
        fs::write(temp.path().join("tree/readonly/a"), "content").unwrap();
        fs::set_permissions(
            temp.path().join("tree/readonly"),
            fs::Permissions::from_mode(0o500),
        )
        .unwrap();
        std::os::unix::fs::symlink("/nonexistent", temp.path().join("tree/link")).unwrap();
        assert!(duplicate(&root, &"tree".parse().unwrap()).await.is_err());
        assert!(!temp.path().join("tree copy").exists());
        assert_eq!(
            fs::metadata(temp.path().join("tree/readonly"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o500
        );
        fs::set_permissions(
            temp.path().join("tree/readonly"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }
}
