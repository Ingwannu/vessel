//! A workspace image's exact bytes in memory, for the Files tab's image
//! panel (F-F40; docs/plans/wp-file-viewers.md): fetched page by page
//! with `file.read_bytes` through the client's one checked loop, as
//! Download fetches (`download.rs`), into memory instead of a file.
//!
//! [Decision Log]
//! - Purpose: Paseo's image panel draws the file; GPUI decodes an image
//!   from its bytes.
//! - Constraint: the node gives a file of at most 64 MiB in pages of at
//!   most 256 KiB, each bound to the first page's version (ADR 0083); the
//!   client checks each page and leaves the whole-file check to the sink.
//! - Alternatives: Download's partial file then a read of it (a file on
//!   this computer for every image looked at); a temporary file handed to
//!   GPUI by path (the same, and its cache keyed by path would keep an old
//!   version).
//! - Choice: a sink that keeps the pages in memory, hashing as they come;
//!   it hands them over only when their SHA-256 is the version's.
//! - Cost: up to the node's 64 MiB per image while it is read and shown.

use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use sha2::{Digest as _, Sha256};
use vessel_app_view::workspace::file_panels::DownloadFailure;
use vessel_client::{ChunkSink, NodeClient};
use vessel_proto::workspace::ContentHash;
use vessel_proto::workspace::panel_files::FileRevision;

use super::ReadImage;
use super::download::failure;

/// An image's bytes and the version they are (`file.read_bytes`, checked
/// whole against its hash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBytes {
    /// The file's bytes.
    pub bytes: Arc<[u8]>,
    /// Their version.
    pub revision: FileRevision,
}

/// Fetch the image `ask` names into memory.
pub(super) async fn fetch(
    client: &NodeClient,
    ask: &ReadImage,
) -> Result<ImageBytes, DownloadFailure> {
    let kept = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&kept);
    let revision = client
        .fetch_workspace_file(ask.root, ask.path.clone(), move |revision| {
            Ok(Memory::new(into, revision))
        })
        .await
        .map_err(|error| failure(&error))?;
    let bytes = std::mem::take(&mut *kept.lock().unwrap_or_else(PoisonError::into_inner));
    Ok(ImageBytes {
        bytes: bytes.into(),
        revision,
    })
}

/// Where the pages go: memory, hashed as it fills.
struct Memory {
    bytes: Vec<u8>,
    hasher: Sha256,
    expected: ContentHash,
    kept: Arc<Mutex<Vec<u8>>>,
}

impl Memory {
    fn new(kept: Arc<Mutex<Vec<u8>>>, revision: &FileRevision) -> Self {
        Self {
            bytes: Vec::new(),
            hasher: Sha256::new(),
            expected: revision.hash.clone(),
            kept,
        }
    }
}

impl ChunkSink for Memory {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.hasher.update(data);
        self.bytes.extend_from_slice(data);
        Ok(())
    }

    fn finish(self) -> io::Result<()> {
        if ContentHash::from_sha256(self.hasher.finalize().into()) != self.expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the bytes are not the version the node named",
            ));
        }
        *self.kept.lock().unwrap_or_else(PoisonError::into_inner) = self.bytes;
        Ok(())
    }
}
