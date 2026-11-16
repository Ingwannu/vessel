//! Where an upload's bytes come from, and what a failure leaves to try.

use std::sync::Arc;

use vessel_proto::{ErrorCode, UploadId, WireError};

use super::*;

fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vessel-app-attach-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn remote(code: ErrorCode) -> RequestFailure {
    RequestFailure::Call(CallError::Remote(WireError {
        code,
        message: "refused".to_owned(),
        detail: None,
    }))
}

fn begun() -> Begun {
    Begun {
        upload: UploadId::random(),
        chunk: 4,
        received: 4,
    }
}

#[tokio::test]
async fn a_file_is_read_a_piece_at_a_time_from_where_the_node_is() {
    let path = scratch("pieces.bin", b"0123456789");
    let origin = Origin::File(path);
    let source = Source::of(&origin);
    assert_eq!(source.head().await.unwrap(), b"0123456789");
    assert_eq!(
        source.piece(4, 4).await.unwrap(),
        Bytes::from_static(b"4567")
    );
    assert_eq!(source.piece(8, 2).await.unwrap(), Bytes::from_static(b"89"));
    assert_eq!(
        source.piece(8, 4).await,
        Err(Problem::Changed),
        "shorter than it was attached"
    );
}

#[tokio::test]
async fn a_pasted_image_is_read_from_memory() {
    let origin = Origin::Pasted(Arc::from(b"abcdef".as_slice()));
    let source = Source::of(&origin);
    assert_eq!(source.head().await.unwrap(), b"abcdef");
    assert_eq!(
        source.piece(2, 3).await.unwrap(),
        Bytes::from_static(b"cde")
    );
    assert_eq!(source.piece(4, 4).await, Err(Problem::Changed));
}

#[tokio::test]
async fn a_missing_file_is_unreadable_and_begun_again() {
    let origin = Origin::File(PathBuf::from("/nonexistent/vessel-attach"));
    let (problem, again) = Source::of(&origin).head().await.unwrap_err();
    assert!(matches!(problem, Problem::Unreadable(_)), "{problem:?}");
    assert_eq!(again, Again::Restart);
}

#[test]
fn a_failure_says_how_it_may_be_tried_again() {
    let at = begun();
    assert_eq!(
        node(&remote(ErrorCode::NotFound), Some(at)).1,
        Again::Restart,
        "the node dropped the upload"
    );
    assert_eq!(
        node(&remote(ErrorCode::InvalidRequest), Some(at)).1,
        Again::Never,
        "the file itself is wrong"
    );
    assert_eq!(
        node(&RequestFailure::Call(CallError::Disconnected), Some(at)).1,
        Again::Resume(at),
        "goes on from what the node has"
    );
    assert_eq!(
        node(&RequestFailure::Call(CallError::Disconnected), None).1,
        Again::Restart,
        "nothing begun yet"
    );
    assert!(matches!(
        node(&remote(ErrorCode::Unavailable), Some(at)).0,
        Problem::Node(_)
    ));
}

#[test]
fn retired_admission_is_never_retryable_and_peer_text_is_redacted() {
    let failure = RequestFailure::Refused(vessel_app_view::connection::Refusal::Changed);
    assert_eq!(node(&failure, Some(begun())).1, Again::Never);
    let secret = "synthetic-private-peer-diagnostic";
    let failure = RequestFailure::Call(CallError::Remote(WireError {
        code: ErrorCode::Internal,
        message: secret.to_owned(),
        detail: None,
    }));
    let (problem, _again) = node(&failure, Some(begun()));
    assert!(!format!("{problem:?}").contains(secret));
    assert!(
        !super::super::selected::problem(&failure)
            .to_string()
            .contains(secret)
    );
}
