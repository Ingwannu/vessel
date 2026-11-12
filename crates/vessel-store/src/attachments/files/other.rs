//! Other platforms (Windows): each path checked with `symlink_metadata`
//! before it is opened, and no file identity, so every read is hashed.

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
