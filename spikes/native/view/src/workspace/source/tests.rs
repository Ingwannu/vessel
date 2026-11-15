//! Complete reads stay on one original root/path and refuse inconsistent peers.
use super::*;
use std::{
    future::ready,
    pin::pin,
    task::{Context, Poll, Waker},
};
use vessel_proto::{
    ProjectId,
    workspace::{ContentHash, FileVersion, RelPath, WorkspaceRoot},
};

fn now<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("ready reader"),
    }
}
fn ask() -> ReadFile {
    ReadFile {
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
        path: "src/a.rs".parse::<RelPath>().unwrap(),
        offset: 0,
        limit: None,
    }
}
fn page(text: &str, size: u64, next: Option<u64>) -> FileRead {
    FileRead {
        version: FileVersion {
            size,
            modified: 1,
            hash: Some("a".repeat(64).parse::<ContentHash>().unwrap()),
        },
        content: FileContent::Text {
            text: text.into(),
            next,
        },
    }
}
#[test]
fn joins_utf8_pages_using_exact_byte_continuations_and_original_scope() {
    let original = ask();
    let mut offsets = Vec::new();
    let result = now(read_source(original.clone(), |request| {
        assert_eq!(request.root, original.root);
        assert_eq!(request.path, original.path);
        offsets.push(request.offset);
        ready(Ok(if request.offset == 0 {
            page("가", 7, Some(3))
        } else {
            page("나!", 7, None)
        }))
    }))
    .unwrap();
    assert_eq!(offsets, [0, 3]);
    assert_eq!(
        result.content,
        FileContent::Text {
            text: "가나!".into(),
            next: None
        }
    );
}
#[test]
fn changed_version_or_retired_reader_never_returns_partial_text() {
    for retired in [false, true] {
        let mut calls = 0;
        let result = now(read_source(ask(), |_| {
            calls += 1;
            ready(if calls == 1 {
                Ok(page("a", 2, Some(1)))
            } else if retired {
                Err(Problem::Gone("original retired".into()))
            } else {
                let mut changed = page("b", 2, None);
                changed.version.modified = 2;
                Ok(changed)
            })
        }));
        assert!(result.is_err());
        assert_eq!(calls, 2);
    }
}
#[test]
fn invalid_progress_and_eof_are_rejected_without_retry() {
    for first in [
        page("", 2, Some(0)),
        page("a", 2, Some(2)),
        page("a", 2, None),
        page("abc", 2, None),
    ] {
        let mut calls = 0;
        assert!(
            now(read_source(ask(), |_| {
                calls += 1;
                ready(Ok(first.clone()))
            }))
            .is_err()
        );
        assert_eq!(calls, 1);
    }
}
#[test]
fn oversize_and_binary_stop_at_the_first_page_and_empty_text_is_valid() {
    let mut binary = page("", 2, None);
    binary.content = FileContent::Binary;
    for first in [
        page("a", MAX_SOURCE_BYTES.saturating_add(1), Some(1)),
        binary,
        page("", 0, None),
    ] {
        let mut calls = 0;
        assert_eq!(
            now(read_source(ask(), |_| {
                calls += 1;
                ready(Ok(first.clone()))
            }))
            .unwrap(),
            first
        );
        assert_eq!(calls, 1);
    }
}
#[test]
fn round_trip_cap_bounds_a_peer_that_advances_one_byte_forever() {
    let mut calls = 0;
    assert!(
        now(read_source(ask(), |request| {
            calls += 1;
            ready(Ok(page(
                "x",
                MAX_SOURCE_BYTES,
                Some(request.offset.saturating_add(1)),
            )))
        }))
        .is_err()
    );
    assert_eq!(calls, MAX_SOURCE_PAGES);
}
