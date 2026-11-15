//! A clock high-water mark cannot stand in for a metadata-field revision.

use super::*;
use vessel_proto::{HarnessKind, SessionId, SessionMode, SessionSpec};

fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    )
}

#[test]
fn delayed_and_unknown_acks_do_not_regress_time_or_discard_unpublished_marks() {
    let first = SessionUpdatedAt::try_from(chrono::DateTime::UNIX_EPOCH).unwrap();
    let later = SessionUpdatedAt::try_from(
        first
            .time()
            .checked_add_signed(SessionUpdatedAt::MIN_INCREMENT)
            .unwrap(),
    )
    .unwrap();
    let mut kept = KeptMetadata::after(None, None, None);
    kept.observe_time(Some(later));
    kept.observe_time(None);
    kept.observe_time(Some(first));
    kept.keep_marks(
        KeptMarks {
            pinned: true,
            ..KeptMarks::default()
        },
        Some(first),
    );
    let mut snapshot = info();
    kept.apply(&mut snapshot);
    assert_eq!(snapshot.updated_at, Some(later));
    assert!(
        snapshot.marks.pinned,
        "a newer append time cannot discard this mark"
    );
}

#[test]
fn legacy_unknown_stays_unknown_until_an_acknowledged_value_exists() {
    let mut kept = KeptMetadata::after(None, None, None);
    kept.observe_time(None);
    let mut snapshot = info();
    kept.apply(&mut snapshot);
    assert_eq!(snapshot.updated_at, None);
}
