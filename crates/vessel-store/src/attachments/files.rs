//! An attachment's bytes on disk: where they are
//! (`attachments/<session>/<id>/<name>`), putting a finished upload there,
//! and reading them back only as the file that was put there.
//!
//! Must not: follow a link anywhere below the data directory's
//! `attachments/`, or hand out bytes of a file that is not the regular file
//! of the size and SHA-256 recorded when it was kept.
//!
//! [Decision Log]
//! - Purpose: a session's agent runs as the node's user and can write in the
//!   node's data directory. It could replace the file it was handed, or a
//!   folder on the way to it, with a link to the node's token; a device that
//!   may only read must then not fetch the token through `attachment.read`
//!   (the contract's review, item 8).
//! - Choice (Unix): every folder is opened relative to the one before with
//!   `O_NOFOLLOW | O_DIRECTORY`, the file with `O_NOFOLLOW | O_NONBLOCK` and
//!   checked to be regular with `fstat`, and a finished upload is moved in
//!   with `renameat` into the folder just made, as `vessel-fleet`'s `dir`
//!   does for a fleet's files, through the same `rustix` (already in the
//!   tree). Then the size and the SHA-256 are compared with the record.
//! - The hash is what makes a swapped file harmless whatever the platform:
//!   a hard link to a secret is a regular file, and only its content gives
//!   it away. Hashing a 50 MiB file for every 1 MiB piece of a download
//!   would hash it fifty times, so a reader may pass the [`FileIdentity`]
//!   it verified last: a file that still has it (same device, inode, size,
//!   modification and change times) is not hashed again. Replacing the file
//!   changes the inode; writing into it changes its change time, which the
//!   writer cannot set back.
//! - Windows: no stable safe API opens relative to a folder handle, so the
//!   path is checked with `symlink_metadata` before it is opened, the
//!   window `vessel-fleet` accepts there too, and there is no identity:
//!   every read is hashed.
//! - Cost: a read checks four path components and hashes the file once per
//!   change; the first read of a 50 MiB file hashes 50 MiB.

use std::fs::File;
use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use vessel_proto::{AttachmentId, BlobHash};

use super::KeptAttachment;

/// Which file was verified: device, inode, size, and modification and
/// change times on Unix. Opaque to callers, who only compare it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

/// Bytes read back from a kept attachment.
#[derive(Debug)]
pub struct ReadBack {
    /// The bytes asked for (none when only a check was asked).
    pub data: Vec<u8>,
    /// The file's identity, verified, to pass to the next read; `None`
    /// where it cannot be known (Windows).
    pub identity: Option<FileIdentity>,
}

/// Why a kept attachment could not be read back, or a finished upload not
/// kept.
#[derive(Debug, thiserror::Error)]
pub enum AttachmentFileError {
    /// Its file is not there.
    #[error("attachment {0}'s file is missing")]
    Missing(AttachmentId),
    /// Its file is not the one that was kept: a link or something other
    /// than a regular file on the way, another size, or other content.
    #[error("attachment {id}'s file was changed on disk ({what}); send it again")]
    Changed {
        /// Which attachment.
        id: AttachmentId,
        /// What differs.
        what: &'static str,
    },
    /// Reading or writing failed.
    #[error("attachment {id}'s file")]
    Io {
        /// Which attachment.
        id: AttachmentId,
        /// What failed.
        #[source]
        source: io::Error,
    },
}

/// Where `kept`'s bytes are under `root` (the data directory's
/// `attachments/`): a folder per session and per attachment, so two
/// `image.png` never collide and the path ends in the file's name. Every
/// component is an id or an `AttachmentName`, one path component each.
pub(crate) fn path(root: &Path, kept: &KeptAttachment) -> PathBuf {
    root.join(kept.session.to_string())
        .join(kept.reference.id.to_string())
        .join(kept.reference.name.as_str())
}

/// A finished upload put in place, until its row is committed: removed
/// again if it is not.
#[derive(Debug)]
pub(crate) struct Placed {
    folder: imp::Dir,
    id: String,
    name: String,
}

impl Placed {
    /// Take the file and its folder away again. Failures are logged; the
    /// start-up sweep removes what is left.
    pub(crate) fn remove(self) {
        let file = self
            .folder
            .open_dir(&self.id)
            .and_then(|dir| dir.remove_file(&self.name));
        let removed = match file {
            // Not moved in: only the folder this made is there.
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            Ok(()) | Err(_) => self.folder.remove_dir(&self.id),
        };
        if let Err(error) = removed {
            tracing::warn!(id = %self.id, %error, "could not take back an attachment not kept");
        }
    }
}

/// Move `staged` (an upload's received bytes) to `kept`'s place under
/// `root`, then check it is the file it should be: the place is made
/// without following a link, and the file there must have `kept`'s size
/// and SHA-256. Blocks.
pub(crate) fn place(
    root: &Path,
    kept: &KeptAttachment,
    staged: &Path,
) -> Result<Placed, AttachmentFileError> {
    let id = kept.reference.id;
    let failed = |source| AttachmentFileError::Io { id, source };
    crate::files::create_private_dir(root).map_err(failed)?;
    let folder = imp::Dir::open(root)
        .and_then(|root| root.ensure_dir(&kept.session.to_string()))
        .map_err(failed)?;
    let placed = Placed {
        id: id.to_string(),
        name: kept.reference.name.as_str().to_owned(),
        folder,
    };
    // A folder of its own: a fresh id, so nothing else is ever there.
    let folder = placed.folder.create_dir(&placed.id).map_err(failed)?;
    if let Err(source) = folder.rename_into(staged, &placed.name) {
        placed.remove();
        return Err(failed(source));
    }
    match read(root, kept, 0, 0, None) {
        Ok(_checked) => Ok(placed),
        Err(error) => {
            placed.remove();
            Err(error)
        }
    }
}

/// Up to `max` bytes of `kept` from `from`, read back only from the regular
/// file of its recorded size and SHA-256, never through a link. A file
/// whose identity is still `verified` is not hashed again. Blocks.
pub(crate) fn read(
    root: &Path,
    kept: &KeptAttachment,
    from: u64,
    max: usize,
    verified: Option<FileIdentity>,
) -> Result<ReadBack, AttachmentFileError> {
    let id = kept.reference.id;
    let changed = |what| AttachmentFileError::Changed { id, what };
    let failed = |source: io::Error| {
        if source.kind() == io::ErrorKind::NotFound {
            AttachmentFileError::Missing(id)
        } else if imp::is_link_refusal(&source) {
            // A link, or a file where a folder should be: `ELOOP`, `ENOTDIR`.
            changed("a link or another kind of file")
        } else {
            AttachmentFileError::Io { id, source }
        }
    };
    let (mut file, identity) = open(root, kept).map_err(failed)?;
    let size = file.metadata().map_err(failed)?.len();
    if size != kept.reference.size {
        return Err(changed("its size"));
    }
    let wanted = usize::try_from(size.saturating_sub(from))
        .unwrap_or(usize::MAX)
        .min(max);
    let known = identity.is_some() && identity == verified;
    let whole = from == 0 && wanted == usize::try_from(size).unwrap_or(usize::MAX);
    let mut data = Vec::new();
    if !known {
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; 64 << 10];
        loop {
            let read = file.read(&mut buffer).map_err(failed)?;
            let Some(piece) = buffer.get(..read).filter(|piece| !piece.is_empty()) else {
                break;
            };
            hasher.update(piece);
            if whole {
                data.extend_from_slice(piece);
            }
        }
        if BlobHash::from_bytes(hasher.finalize().into()) != kept.sha256 {
            return Err(changed("its content"));
        }
    }
    if data.len() != wanted {
        data.clear();
        let _at = file.seek(SeekFrom::Start(from)).map_err(failed)?;
        let mut piece = (&mut file).take(u64::try_from(wanted).unwrap_or(u64::MAX));
        let _read = piece.read_to_end(&mut data).map_err(failed)?;
        if data.len() != wanted {
            return Err(changed("its size"));
        }
    }
    Ok(ReadBack { data, identity })
}

/// `kept`'s file, opened without following a link on the way, and its
/// identity.
fn open(root: &Path, kept: &KeptAttachment) -> io::Result<(File, Option<FileIdentity>)> {
    let file = imp::Dir::open(root)?
        .open_dir(&kept.session.to_string())?
        .open_dir(&kept.reference.id.to_string())?
        .open_file(kept.reference.name.as_str())?;
    let identity = imp::identity(&file.metadata()?);
    Ok((file, identity))
}

#[cfg(unix)]
mod imp {
    use std::fs::File;
    use std::io;
    use std::os::fd::OwnedFd;
    use std::os::unix::fs::MetadataExt as _;
    use std::path::Path;

    use rustix::fs::{AtFlags, CWD, Mode, OFlags};
    use rustix::io::Errno;

    use super::FileIdentity;

    const DIRECTORY: OFlags = OFlags::RDONLY
        .union(OFlags::DIRECTORY)
        .union(OFlags::NOFOLLOW)
        .union(OFlags::CLOEXEC);

    /// An open folder; everything below it is named relative to it.
    #[derive(Debug)]
    pub(super) struct Dir(OwnedFd);

    impl Dir {
        /// `path` itself must not be a link; the folders above it are the
        /// data directory's, which the node made.
        pub(super) fn open(path: &Path) -> io::Result<Self> {
            Ok(Self(rustix::fs::openat(
                CWD,
                path,
                DIRECTORY,
                Mode::empty(),
            )?))
        }

        pub(super) fn open_dir(&self, name: &str) -> io::Result<Self> {
            Ok(Self(rustix::fs::openat(
                &self.0,
                name,
                DIRECTORY,
                Mode::empty(),
            )?))
        }

        /// The folder `name`, made owner-only if it is not there.
        pub(super) fn ensure_dir(&self, name: &str) -> io::Result<Self> {
            match rustix::fs::mkdirat(&self.0, name, Mode::RWXU) {
                Ok(()) | Err(Errno::EXIST) => self.open_dir(name),
                Err(error) => Err(error.into()),
            }
        }

        /// A new owner-only folder `name`; an error if anything is there.
        pub(super) fn create_dir(&self, name: &str) -> io::Result<Self> {
            rustix::fs::mkdirat(&self.0, name, Mode::RWXU)?;
            self.open_dir(name)
        }

        /// The regular file `name`; an error for a link or anything else.
        pub(super) fn open_file(&self, name: &str) -> io::Result<File> {
            let flags = OFlags::RDONLY
                .union(OFlags::NOFOLLOW)
                .union(OFlags::NONBLOCK)
                .union(OFlags::CLOEXEC);
            let file = File::from(rustix::fs::openat(&self.0, name, flags, Mode::empty())?);
            if !file.metadata()?.file_type().is_file() {
                return Err(Errno::LOOP.into());
            }
            Ok(file)
        }

        /// Move the file at `from` here as `name`.
        pub(super) fn rename_into(&self, from: &Path, name: &str) -> io::Result<()> {
            Ok(rustix::fs::renameat(CWD, from, &self.0, name)?)
        }

        pub(super) fn remove_file(&self, name: &str) -> io::Result<()> {
            Ok(rustix::fs::unlinkat(&self.0, name, AtFlags::empty())?)
        }

        pub(super) fn remove_dir(&self, name: &str) -> io::Result<()> {
            Ok(rustix::fs::unlinkat(&self.0, name, AtFlags::REMOVEDIR)?)
        }
    }

    /// A link (`ELOOP`) or a file where a folder was expected (`ENOTDIR`).
    pub(super) fn is_link_refusal(error: &io::Error) -> bool {
        let code = Errno::from_io_error(error);
        code == Some(Errno::LOOP) || code == Some(Errno::NOTDIR)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "one signature on every platform: Windows has no identity"
    )]
    pub(super) fn identity(metadata: &std::fs::Metadata) -> Option<FileIdentity> {
        Some(FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.size(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

#[cfg(not(unix))]
mod imp {
    use std::fs::{self, File};
    use std::io;
    use std::path::{Path, PathBuf};

    use super::FileIdentity;

    /// A link, or something other than what was expected.
    #[derive(Debug, thiserror::Error)]
    #[error("a link or another kind of file")]
    struct Refused;

    fn refused() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, Refused)
    }

    /// A folder, checked when it was opened (see the module docs).
    #[derive(Debug)]
    pub(super) struct Dir(PathBuf);

    impl Dir {
        pub(super) fn open(path: &Path) -> io::Result<Self> {
            if !fs::symlink_metadata(path)?.file_type().is_dir() {
                return Err(refused());
            }
            Ok(Self(path.to_path_buf()))
        }

        pub(super) fn open_dir(&self, name: &str) -> io::Result<Self> {
            Self::open(&self.0.join(name))
        }

        pub(super) fn ensure_dir(&self, name: &str) -> io::Result<Self> {
            match fs::create_dir(self.0.join(name)) {
                Err(error) if error.kind() != io::ErrorKind::AlreadyExists => Err(error),
                Ok(()) | Err(_) => self.open_dir(name),
            }
        }

        pub(super) fn create_dir(&self, name: &str) -> io::Result<Self> {
            fs::create_dir(self.0.join(name))?;
            self.open_dir(name)
        }

        pub(super) fn open_file(&self, name: &str) -> io::Result<File> {
            let path = self.0.join(name);
            if !fs::symlink_metadata(&path)?.file_type().is_file() {
                return Err(refused());
            }
            let file = File::open(path)?;
            if !file.metadata()?.file_type().is_file() {
                return Err(refused());
            }
            Ok(file)
        }

        pub(super) fn rename_into(&self, from: &Path, name: &str) -> io::Result<()> {
            fs::rename(from, self.0.join(name))
        }

        pub(super) fn remove_file(&self, name: &str) -> io::Result<()> {
            fs::remove_file(self.0.join(name))
        }

        pub(super) fn remove_dir(&self, name: &str) -> io::Result<()> {
            fs::remove_dir(self.0.join(name))
        }
    }

    pub(super) fn is_link_refusal(error: &io::Error) -> bool {
        error.get_ref().is_some_and(|inner| inner.is::<Refused>())
    }

    pub(super) const fn identity(_metadata: &std::fs::Metadata) -> Option<FileIdentity> {
        None
    }
}
