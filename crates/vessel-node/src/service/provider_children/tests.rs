//! The actual scan edge refuses elapsed admission and reports real oversized boundaries.
use super::*;
use vessel_proto::subagents::{ChildGap, ChildHistory};
use vessel_proto::{AgentEvent, EventRecord, SessionEvent};
fn record(seq: u64, line: String) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::HarnessLog { line },
        },
    }
}
#[test]
fn elapsed_before_first_observation_is_refused_without_an_invented_partial_sequence() {
    let mut projection = projection::Projection::new(None, EventSeq::ZERO, EventSeq::new(1));
    let result = observe_page(
        &mut projection,
        &mut ChildScanBudget::default(),
        vec![record(0, "unread".to_owned())],
        EventSeq::new(1),
        tokio::time::Instant::now(),
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::Unavailable);
    assert!(!projection.scanned());
}
#[test]
fn an_oversized_first_record_is_partial_at_the_actual_decoded_parent_sequence() {
    let mut projection = projection::Projection::new(None, EventSeq::new(40), EventSeq::new(50));
    assert!(
        !observe_page(
            &mut projection,
            &mut ChildScanBudget::default(),
            vec![record(43, "x".repeat(bounds::MAX_SCAN_BYTES))],
            EventSeq::new(50),
            tokio::time::Instant::now()
                .checked_add(bounds::READ_DEADLINE)
                .unwrap()
        )
        .unwrap()
    );
    assert!(matches!(projection.list().unwrap().history,
        ChildHistory::Partial { reason: ChildGap::ScanBudget, from } if from == EventSeq::new(43)));
}
#[test]
fn elapsed_after_observation_keeps_the_scanned_boundary_and_never_reads_another_record() {
    let mut projection = projection::Projection::new(None, EventSeq::new(40), EventSeq::new(50));
    let _continued = projection.observe(record(40, "read".to_owned()));
    assert!(
        !observe_page(
            &mut projection,
            &mut ChildScanBudget::default(),
            vec![record(41, "unread".to_owned())],
            EventSeq::new(50),
            tokio::time::Instant::now()
        )
        .unwrap()
    );
    assert_eq!(projection.next(), EventSeq::new(41));
    assert!(matches!(projection.list().unwrap().history,
        ChildHistory::Partial { reason: ChildGap::ScanBudget, from } if from == EventSeq::new(40)));
}
