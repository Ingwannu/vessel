//! A file of a root kept on this computer (Paseo's Explorer "Download",
//! ADR 0105): fetched page by page with `file.read_bytes` through the
//! client's one checked loop, into a file beside the chosen one that
//! takes the chosen name only when it is the whole version.
//!
//! [Decision Log]
//! - Purpose: a copy of a project file on this computer, as Paseo's
//!   desktop download keeps one where the person says.
//! - Constraint: the node gives a file of at most 64 MiB in pages of at
//!   most 256 KiB, each bound to the first page's version (ADR 0083); the
//!   client's `fetch_workspace_file` checks each page and leaves the
//!   whole-file check and the keeping to the sink.
//! - Alternatives: hold the file in memory and write it at the end (up to
//!   64 MiB per download); write straight to the chosen path (a failed or
//!   changed transfer would leave a broken file where the person expects
//!   theirs).
//! - Choice: a sink that writes a hidden partial file in the chosen
//!   folder, hashing as it writes; at the end it is renamed over the chosen
//!   path only if its SHA-256 is the version's. Anything else removes it.
//! - Cost: the folder needs room for the partial file beside an old one it
//!   replaces; a rename over an existing file replaces it (the save dialog
//!   asked). The hash runs once more over bytes the node already hashed.

use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::{self, Write as _};
use std::path::PathBuf;

use sha2::{Digest as _, Sha256};
use vessel_app_view::workspace::file_panels::DownloadFailure;
use vessel_client::{
    ChunkSink, FetchError, NodeClient, WorkspaceFileFetchError, WorkspaceFilePageError,
};
use vessel_proto::workspace::ContentHash;
use vessel_proto::workspace::panel_files::FileRevision;

use super::Download;
use super::calls::file_problem;

/// Partial names tried before a download gives up (another download of
/// the same name may hold one).
const PARTIAL_NAMES: u32 = 16;

/// Fetch `download` into its chosen path.
pub(super) async fn fetch(client: &NodeClient, download: &Download) -> Result<(), DownloadFailure> {
    let to = download.to.clone();
    client
        .fetch_workspace_file(download.root, download.path.clone(), move |revision| {
            Output::create(to, revision)
        })
        .await
        .map(drop)
        .map_err(|error| failure(&error))
}

/// A fetch's error as a download's failure.
fn failure(error: &WorkspaceFileFetchError) -> DownloadFailure {
    match error {
        WorkspaceFileFetchError::Read(error)
        | WorkspaceFileFetchError::Transfer(FetchError::Read(WorkspaceFilePageError::Call(
            error,
        ))) => DownloadFailure::Call(file_problem(error)),
        WorkspaceFileFetchError::Transfer(FetchError::Sink(error))
            if error.kind() == io::ErrorKind::InvalidData =>
        {
            DownloadFailure::Inconsistent
        }
        WorkspaceFileFetchError::Page(_)
        | WorkspaceFileFetchError::Hash(_)
        | WorkspaceFileFetchError::Transfer(
            FetchError::Read(WorkspaceFilePageError::Page(_)) | FetchError::Oversized(_),
        ) => DownloadFailure::Inconsistent,
        WorkspaceFileFetchError::Sink(error)
        | WorkspaceFileFetchError::Transfer(FetchError::Sink(error)) => {
            DownloadFailure::Local(error.to_string())
        }
        WorkspaceFileFetchError::Transfer(FetchError::Task(error)) => {
            DownloadFailure::Local(error.to_string())
        }
    }
}

/// A partial file, removed unless it was kept.
struct Partial {
    path: PathBuf,
    kept: bool,
}

impl Drop for Partial {
    fn drop(&mut self) {
        if !self.kept {
            // Nothing to tell anyone: a partial that cannot be removed is
            // a hidden file the person may delete; the download's own
            // error is what is reported.
            let _removed = std::fs::remove_file(&self.path);
        }
    }
}

/// Where the pages go: the partial file, hashed as it is written.
struct Output {
    file: File,
    hasher: Sha256,
    expected: ContentHash,
    to: PathBuf,
    partial: Partial,
}

impl Output {
    /// A new partial file beside `to` (never an existing file), for the
    /// version `revision`.
    fn create(to: PathBuf, revision: &FileRevision) -> io::Result<Self> {
        let (Some(folder), Some(name)) = (to.parent(), to.file_name()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a download needs a file name in a folder",
            ));
        };
        let mut attempt = 0_u32;
        loop {
            let path = folder.join(partial_name(name, attempt));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        file,
                        hasher: Sha256::new(),
                        expected: revision.hash.clone(),
                        to,
                        partial: Partial { path, kept: false },
                    });
                }
                Err(error)
                    if error.kind() == io::ErrorKind::AlreadyExists && attempt < PARTIAL_NAMES =>
                {
                    attempt = attempt.saturating_add(1);
                }
                Err(error) => return Err(error),
            }
        }
    }
}

/// `.name.N.download`: hidden, beside the chosen name.
fn partial_name(name: &OsStr, attempt: u32) -> OsString {
    let mut partial = OsString::from(".");
    partial.push(name);
    partial.push(format!(".{attempt}.download"));
    partial
}

impl ChunkSink for Output {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.hasher.update(data);
        self.file.write_all(data)
    }

    fn finish(self) -> io::Result<()> {
        let Self {
            mut file,
            hasher,
            expected,
            to,
            mut partial,
        } = self;
        file.flush()?;
        file.sync_all()?;
        // Closed before the rename: Windows does not rename an open file.
        drop(file);
        if ContentHash::from_sha256(hasher.finalize().into()) != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the bytes downloaded are not the file's version",
            ));
        }
        std::fs::rename(&partial.path, &to)?;
        partial.kept = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace::panel_files::FileSize;

    fn revision(bytes: &[u8]) -> FileRevision {
        FileRevision {
            size: FileSize(u64::try_from(bytes.len()).unwrap()),
            hash: ContentHash::from_sha256(Sha256::digest(bytes).into()),
        }
    }

    /// A folder of the test's own under the system's temporary folder,
    /// removed when dropped.
    struct Folder(PathBuf);

    impl Folder {
        fn new(test: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "vessel-link-download-{}-{test}",
                std::process::id()
            ));
            let _stale = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _removed = std::fs::remove_dir_all(&self.0);
        }
    }

    fn partials(folder: &std::path::Path) -> Vec<OsString> {
        std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().ends_with(".download"))
            .collect()
    }

    #[test]
    fn the_whole_version_takes_the_chosen_name_over_an_old_file() {
        let folder = Folder::new("whole");
        let to = folder.path().join("notes.txt");
        std::fs::write(&to, b"old").unwrap();
        let mut output = Output::create(to.clone(), &revision(b"hello\n")).unwrap();
        output.write(b"hel").unwrap();
        output.write(b"lo\n").unwrap();
        output.finish().unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"hello\n");
        assert!(partials(folder.path()).is_empty());
    }

    #[test]
    fn other_bytes_or_an_unfinished_download_leave_nothing() {
        let folder = Folder::new("other");
        let to = folder.path().join("notes.txt");
        let mut output = Output::create(to.clone(), &revision(b"hello\n")).unwrap();
        output.write(b"HELLO\n").unwrap();
        let error = output.finish().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!to.exists());
        let mut dropped = Output::create(to.clone(), &revision(b"hello\n")).unwrap();
        dropped.write(b"he").unwrap();
        drop(dropped);
        assert!(!to.exists());
        assert!(partials(folder.path()).is_empty());
    }

    #[test]
    fn a_partial_name_in_use_takes_the_next() {
        let folder = Folder::new("next");
        let to = folder.path().join("notes.txt");
        let first = Output::create(to.clone(), &revision(b"a")).unwrap();
        let second = Output::create(to, &revision(b"a")).unwrap();
        assert_ne!(first.partial.path, second.partial.path);
        assert_eq!(partials(folder.path()).len(), 2);
    }

    #[test]
    fn a_sink_hash_refusal_is_an_inconsistent_download() {
        let refused = WorkspaceFileFetchError::Transfer(FetchError::Sink(io::Error::new(
            io::ErrorKind::InvalidData,
            "the bytes downloaded are not the file's version",
        )));
        assert_eq!(failure(&refused), DownloadFailure::Inconsistent);
        let full = WorkspaceFileFetchError::Transfer(FetchError::Sink(io::Error::new(
            io::ErrorKind::StorageFull,
            "no space",
        )));
        assert!(matches!(failure(&full), DownloadFailure::Local(_)));
    }
}
