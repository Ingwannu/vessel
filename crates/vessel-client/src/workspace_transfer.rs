//! Workspace downloads reuse the existing chunk sink/transfer loop.
//! Page identity and continuation are checked before bytes reach a sink;
//! its finish contract verifies the complete hash before publishing output.

use std::future::{Future, ready};
use std::io;

use futures_util::future::Either;
use vessel_proto::workspace::panel_files::{
    FileBytePage, FileOffset, FileRevision, MAX_FILE_BYTES, ReadFileBytes,
};
use vessel_proto::workspace::{RelPath, WorkspaceRoot};
use vessel_proto::{BlobChunk, BundleFile};

use crate::{ChunkSink, FetchError, NodeClient, WorkspaceFileCallError, fetch_chunks};

/// A peer's bytes do not describe one complete, coherent source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidFilePage {
    /// The complete source exceeds the file-panel limit.
    #[error("file source exceeds the transfer limit")]
    TooLarge,
    /// A continuation belongs to different content.
    #[error("file changed during transfer")]
    Revision,
    /// The reply starts at another byte or ends beyond the declared size.
    #[error("file page offset does not match the transfer")]
    Offset,
    /// EOF/next disagrees with the exact byte count.
    #[error("file transfer continuation is inconsistent")]
    Continuation,
}

/// Call errors keep their source; invalid peers never publish a partial file.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceFilePageError {
    /// Node admission, transport or filesystem failure.
    #[error(transparent)]
    Call(#[from] WorkspaceFileCallError),
    /// Incoherent response data.
    #[error(transparent)]
    Page(#[from] InvalidFilePage),
}

/// Fetching into a caller's hash-checking, atomic output sink failed.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceFileFetchError {
    /// Initial node read failed before creating output.
    #[error(transparent)]
    Read(#[from] WorkspaceFileCallError),
    /// Initial page is invalid.
    #[error(transparent)]
    Page(#[from] InvalidFilePage),
    /// A content identity could not be converted to the shared sink manifest.
    #[error("file content hash is invalid")]
    Hash(#[source] vessel_proto::KeyTextError),
    /// A temporary sink could not be created.
    #[error("cannot create download output")]
    Sink(#[source] io::Error),
    /// A later read, sink write, whole hash check or worker failed.
    #[error(transparent)]
    Transfer(#[from] FetchError<WorkspaceFilePageError>),
}

impl NodeClient {
    /// Download one bounded file into an output that validates its whole hash
    /// against `revision` before publishing. `make_sink` is called only after
    /// the first page and its declared size have been validated.
    ///
    /// # Errors
    /// Refuses mixed, truncated, misrouted or unsupported transfers. A failed
    /// transfer drops its unfinished sink instead of keeping partial output.
    pub async fn fetch_workspace_file<S: ChunkSink>(
        &self,
        root: WorkspaceRoot,
        path: RelPath,
        make_sink: impl FnOnce(&FileRevision) -> io::Result<S> + Send + 'static,
    ) -> Result<FileRevision, WorkspaceFileFetchError> {
        fetch_workspace_file_with(
            root,
            path,
            |request| self.read_workspace_byte_page(request),
            make_sink,
        )
        .await
    }
}

/// Fetch through the caller's original admitted reader, reusing the same
/// validated transfer and sink loop. The caller enforces connection admission
/// for every page; constructing this reader grants no filesystem authority.
///
/// # Errors
/// As [`NodeClient::fetch_workspace_file`]. An admission refusal at any page
/// drops unfinished output; initial refusal never constructs a sink.
pub async fn fetch_workspace_file_with<S, R, F>(
    root: WorkspaceRoot,
    path: RelPath,
    mut read: R,
    make_sink: impl FnOnce(&FileRevision) -> io::Result<S> + Send + 'static,
) -> Result<FileRevision, WorkspaceFileFetchError>
where
    S: ChunkSink,
    R: FnMut(ReadFileBytes) -> F,
    F: Future<Output = Result<FileBytePage, WorkspaceFileCallError>>,
{
    let first = read(ReadFileBytes {
        root,
        path: path.clone(),
        offset: FileOffset::ZERO,
        expected: None,
    })
    .await?;
    let _validated = validate_page(&first, &first.revision, 0)?;
    let creating = first.revision.clone();
    let sink = tokio::task::spawn_blocking(move || make_sink(&creating))
        .await
        .map_err(|error| WorkspaceFileFetchError::Transfer(FetchError::Task(error)))?
        .map_err(WorkspaceFileFetchError::Sink)?;
    fetch_pages(first, sink, move |offset, expected| {
        read(ReadFileBytes {
            root,
            path: path.clone(),
            offset,
            expected: Some(expected),
        })
    })
    .await
}

fn validate_page(
    page: &FileBytePage,
    expected: &FileRevision,
    offset: u64,
) -> Result<BlobChunk, InvalidFilePage> {
    if expected.size.0 > MAX_FILE_BYTES {
        return Err(InvalidFilePage::TooLarge);
    }
    if page.revision != *expected {
        return Err(InvalidFilePage::Revision);
    }
    let end = offset
        .checked_add(
            u64::try_from(page.bytes.bytes().len()).map_err(|_overflow| InvalidFilePage::Offset)?,
        )
        .ok_or(InvalidFilePage::Offset)?;
    if page.offset.get() != offset || end > expected.size.0 {
        return Err(InvalidFilePage::Offset);
    }
    let next = if end < expected.size.0 {
        if page.bytes.bytes().is_empty() {
            return Err(InvalidFilePage::Continuation);
        }
        Some(end)
    } else {
        None
    };
    if page.next.map(FileOffset::get) != next {
        return Err(InvalidFilePage::Continuation);
    }
    Ok(BlobChunk {
        data: page.bytes.bytes().clone(),
        total: expected.size.0,
    })
}

async fn fetch_pages<F, S>(
    first: FileBytePage,
    sink: S,
    mut read: impl FnMut(FileOffset, FileRevision) -> F,
) -> Result<FileRevision, WorkspaceFileFetchError>
where
    F: Future<Output = Result<FileBytePage, WorkspaceFileCallError>>,
    S: ChunkSink,
{
    let revision = first.revision.clone();
    let manifest = BundleFile {
        hash: revision
            .hash
            .as_str()
            .parse()
            .map_err(WorkspaceFileFetchError::Hash)?,
        size: revision.size.0,
        executable: false,
    };
    let mut first = Some(first);
    fetch_chunks(&manifest, sink, |offset| {
        let expected = revision.clone();
        let reading = FileOffset::try_from(offset).map(|position| match first.take() {
            Some(page) => Either::Left(ready(Ok(page))),
            None => Either::Right(read(position, expected.clone())),
        });
        async move {
            let reading = reading.map_err(|_invalid| InvalidFilePage::Offset)?;
            let page = reading.await?;
            Ok::<BlobChunk, WorkspaceFilePageError>(validate_page(&page, &expected, offset)?)
        }
    })
    .await?;
    Ok(revision)
}

#[cfg(test)]
mod tests;
