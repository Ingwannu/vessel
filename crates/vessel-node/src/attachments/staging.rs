//! An upload's staging file under the data directory's `uploads/`: made
//! owner-only at `begin`, appended to piece by piece while its SHA-256 and
//! first bytes are kept, removed when the upload is dropped, and emptied
//! at every start.

use std::fs::File;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use vessel_proto::BlobHash;
use vessel_store::DataDir;

/// How many of an upload's first bytes are kept, for an image's check.
const HEAD: usize = 12;

/// An upload's bytes so far.
#[derive(Debug)]
pub(crate) struct Progress {
    file: File,
    hasher: Sha256,
    head: Vec<u8>,
    pub(crate) received: u64,
}

impl Progress {
    /// Nothing received yet, into `file`.
    pub(crate) fn new(file: File) -> Self {
        Self {
            file,
            hasher: Sha256::new(),
            head: Vec::with_capacity(HEAD),
            received: 0,
        }
    }

    /// Append `data`. Blocks.
    pub(crate) fn write(mut self, data: &[u8]) -> io::Result<Self> {
        self.file.write_all(data)?;
        self.hasher.update(data);
        let room = HEAD.saturating_sub(self.head.len());
        self.head
            .extend_from_slice(data.get(..room.min(data.len())).unwrap_or_default());
        self.received = self
            .received
            .saturating_add(u64::try_from(data.len()).unwrap_or(u64::MAX));
        Ok(self)
    }

    /// Its first bytes, for an image's check.
    pub(crate) fn head(&self) -> &[u8] {
        &self.head
    }

    /// Close the file; the SHA-256 of what came.
    pub(crate) fn finish(self) -> BlobHash {
        drop(self.file);
        BlobHash::from_bytes(self.hasher.finalize().into())
    }
}

/// `<data_dir>/uploads`.
pub(crate) fn root(data_dir: &DataDir) -> PathBuf {
    data_dir.uploads_dir()
}

/// Empty the staging directory: uploads open at a restart are dropped
/// (protocol 10). Blocks; called before the node serves.
pub(crate) fn clear(data_dir: &DataDir) {
    let root = root(data_dir);
    match std::fs::remove_dir_all(&root) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            tracing::warn!(path = %root.display(), %error, "could not empty the uploads' staging directory");
        }
        Ok(()) | Err(_) => {}
    }
}

/// A new owner-only staging file at `path`, its folder made if needed.
pub(crate) fn stage(root: &Path, path: &Path) -> io::Result<File> {
    let mut folder = std::fs::DirBuilder::new();
    let mut file = std::fs::OpenOptions::new();
    let _new = file.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
        let _private = folder.mode(0o700);
        let _private = file.mode(0o600);
    }
    folder.recursive(true).create(root)?;
    file.open(path)
}

/// Remove a staging file; failures are logged (the next start empties the
/// staging directory anyway).
pub(crate) fn discard(path: &Path) {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            tracing::warn!(path = %path.display(), %error, "could not remove an upload's staging file");
        }
        Ok(()) | Err(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_is_hashed_and_its_head_kept() {
        let temp = tempfile::tempdir().unwrap();
        let file = stage(temp.path(), &temp.path().join("u")).unwrap();
        let progress = Progress::new(file);
        let progress = progress.write(b"\x89PNG\r\n").unwrap();
        let progress = progress.write(b"\x1a\nmore than twelve bytes").unwrap();
        assert_eq!(progress.head(), b"\x89PNG\r\n\x1a\nmore");
        assert_eq!(progress.received, 30);
        let hash = progress.finish();
        assert_eq!(
            hash,
            BlobHash::from_bytes(Sha256::digest(b"\x89PNG\r\n\x1a\nmore than twelve bytes").into())
        );
        assert_eq!(
            std::fs::read(temp.path().join("u")).unwrap().len(),
            30,
            "on disk"
        );
    }
}
