//! Page boundaries are stable across insertions; cursor lookup does not cross sessions.
use super::*;
use crate::checkpoint::{install_staged, journal, publication};
use chrono::DateTime;
use vessel_proto::{
    EventSeq, WorktreeId,
    checkpoint::{CaptureKey, FileImage},
};
fn fixture() -> (Connection, CaptureKey) {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE sessions(id TEXT PRIMARY KEY);")
        .unwrap();
    install_staged(&connection).unwrap();
    let key = CaptureKey {
        capture: CaptureId::random(),
        session: SessionId::random(),
        worktree: WorktreeId::random(),
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::new(0),
        },
        generation: CaptureGeneration::FIRST,
    };
    connection
        .execute(
            "INSERT INTO sessions(id) VALUES (?1)",
            [key.session.to_string()],
        )
        .unwrap();
    (connection, key)
}
fn reserve(connection: &Connection, key: CaptureKey) {
    journal::candidate(
        connection,
        key,
        crate::ByteCount::new(1),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
}
#[test]
fn checkpoint_pages_have_no_duplicates_at_a_tied_timestamp_and_bind_cursor_session() {
    let (connection, mut key) = fixture();
    let count = CHECKPOINT_PAGE_ENTRIES.saturating_add(1);
    for prompt in 0..count {
        key.capture = CaptureId::random();
        key.boundary = Boundary::BeforeTurn {
            prompt: EventSeq::new(u64::try_from(prompt).unwrap()),
        };
        reserve(&connection, key);
    }
    let first = page(&connection, key.session, None).unwrap();
    assert_eq!(first.marks.len(), CHECKPOINT_PAGE_ENTRIES);
    let cursor = first.next.unwrap();
    let second = page(&connection, key.session, Some(cursor)).unwrap();
    assert_eq!(second.marks.len(), 1);
    assert_eq!(second.next, None);
    assert!(!first.marks.iter().any(|mark| second.marks.contains(mark)));
    assert!(matches!(
        page(&connection, SessionId::random(), Some(cursor)),
        Err(Error::Refused(CheckpointError::InvalidCursor))
    ));
    assert!(matches!(
        page(
            &connection,
            key.session,
            Some(CheckpointCursor(CaptureId::random()))
        ),
        Err(Error::Refused(CheckpointError::InvalidCursor))
    ));
}
#[test]
fn checkpoint_ready_and_unavailable_marks_are_durable_metadata_without_blob_reads() {
    let (connection, key) = fixture();
    reserve(&connection, key);
    assert_eq!(
        page(&connection, key.session, None)
            .unwrap()
            .marks
            .first()
            .unwrap()
            .state,
        CheckpointState::Unavailable {
            reason: CheckpointError::Busy
        }
    );
    let image = FileImage::new(Vec::new()).unwrap();
    journal::keep_candidate(&connection, key, &crate::checkpoint::tests::observed(image)).unwrap();
    let checkpoint = CheckpointId::random();
    publication::activate(&connection, key, checkpoint).unwrap();
    assert_eq!(
        page(&connection, key.session, None)
            .unwrap()
            .marks
            .first()
            .unwrap()
            .state,
        CheckpointState::Ready { checkpoint }
    );
    let mut failed = key;
    failed.capture = CaptureId::random();
    failed.boundary = Boundary::AfterTurn {
        ended: EventSeq::new(1),
    };
    reserve(&connection, failed);
    journal::unavailable(&connection, failed, CheckpointError::UnsupportedMetadata).unwrap();
    let listed = page(&connection, key.session, None).unwrap();
    assert_eq!(listed.marks.len(), 2);
    assert!(listed.marks.iter().any(|mark| mark.state
        == CheckpointState::Unavailable {
            reason: CheckpointError::UnsupportedMetadata
        }));
}
