//! A malformed page must never publish an unfinished download.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vessel_proto::workspace::ContentHash;
use vessel_proto::workspace::panel_files::{FileBytes, FileSize};

struct Sink {
    bytes: Vec<u8>,
    expected: &'static [u8],
    kept: Arc<AtomicBool>,
}
impl ChunkSink for Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.bytes.extend_from_slice(data);
        Ok(())
    }
    fn finish(self) -> io::Result<()> {
        if self.bytes != self.expected {
            return Err(io::Error::other("incomplete or corrupt fixture"));
        }
        self.kept.store(true, Ordering::SeqCst);
        Ok(())
    }
}
fn sink(expected: &'static [u8]) -> (Sink, Arc<AtomicBool>) {
    let kept = Arc::new(AtomicBool::new(false));
    (
        Sink {
            bytes: Vec::new(),
            expected,
            kept: Arc::clone(&kept),
        },
        kept,
    )
}
fn revision(size: u64) -> FileRevision {
    FileRevision {
        size: FileSize(size),
        hash: ContentHash::from_sha256([7; 32]),
    }
}
fn page(
    revision: &FileRevision,
    offset: u64,
    bytes: &'static [u8],
    next: Option<u64>,
) -> FileBytePage {
    FileBytePage {
        revision: revision.clone(),
        offset: FileOffset::try_from(offset).unwrap(),
        bytes: FileBytes::new(bytes.into()).unwrap(),
        next: next.map(|offset| FileOffset::try_from(offset).unwrap()),
    }
}

#[tokio::test]
async fn valid_pages_use_one_sink_and_publish_only_after_complete_content() {
    let revision = revision(4);
    let first = page(&revision, 0, b"ab", Some(2));
    let second = page(&revision, 2, b"cd", None);
    let (sink, kept) = sink(b"abcd");
    let read = move |offset: FileOffset, expected: FileRevision| {
        assert_eq!(offset.get(), 2);
        assert_eq!(second.revision, expected);
        ready(Ok(second.clone()))
    };
    assert_eq!(fetch_pages(first, sink, read).await.unwrap(), revision);
    assert!(kept.load(Ordering::SeqCst));
}

#[tokio::test]
async fn changed_continuation_and_sink_failure_never_publish() {
    let initial = revision(4);
    let mut changed = page(&initial, 2, b"cd", None);
    changed.revision.hash = ContentHash::from_sha256([8; 32]);
    let (sink, kept) = sink(b"abcd");
    let result = fetch_pages(page(&initial, 0, b"ab", Some(2)), sink, move |_, _| {
        ready(Ok(changed.clone()))
    })
    .await;
    assert!(matches!(
        result,
        Err(WorkspaceFileFetchError::Transfer(FetchError::Read(
            WorkspaceFilePageError::Page(InvalidFilePage::Revision)
        )))
    ));
    assert!(!kept.load(Ordering::SeqCst));
    let (sink, kept) = self::sink(b"must match expected content");
    let result = fetch_pages(page(&revision(2), 0, b"ab", None), sink, |_, _| {
        ready(Err(WorkspaceFileCallError::Deadline))
    })
    .await;
    assert!(matches!(
        result,
        Err(WorkspaceFileFetchError::Transfer(FetchError::Sink(_)))
    ));
    assert!(!kept.load(Ordering::SeqCst));
}

#[test]
fn validates_offsets_size_and_exact_continuation_before_sink_write() {
    let revision = revision(4);
    for (malformed, wanted) in [
        (page(&revision, 1, b"ab", Some(3)), InvalidFilePage::Offset),
        (
            page(&revision, 0, b"ab", None),
            InvalidFilePage::Continuation,
        ),
        (
            page(&revision, 0, b"", Some(0)),
            InvalidFilePage::Continuation,
        ),
        (page(&revision, 0, b"abcde", None), InvalidFilePage::Offset),
        (
            page(&revision, 0, b"ab", Some(3)),
            InvalidFilePage::Continuation,
        ),
    ] {
        assert_eq!(validate_page(&malformed, &revision, 0), Err(wanted));
    }
    let huge = self::revision(MAX_FILE_BYTES + 1);
    assert_eq!(
        validate_page(&page(&huge, 0, b"", None), &huge, 0),
        Err(InvalidFilePage::TooLarge)
    );
}

#[tokio::test]
async fn an_empty_file_finishes_without_asking_for_another_page() {
    let empty = revision(0);
    let (sink, kept) = sink(b"");
    let result = fetch_pages(page(&empty, 0, b"", None), sink, |_, _| {
        ready(Err(WorkspaceFileCallError::Deadline))
    })
    .await;
    assert_eq!(result.unwrap(), empty);
    assert!(kept.load(Ordering::SeqCst));
}
