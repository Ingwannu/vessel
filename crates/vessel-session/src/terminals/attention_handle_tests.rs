//! Handle-level stale clear/producer and once-only races, without processes.
use super::handle::{Shared, TerminalHandle};
use crate::ring::Ring;
use std::sync::{Arc, Mutex, atomic::AtomicU32};
use vessel_proto::notifications::{
    NotificationClaim, TerminalActivitySignal, TerminalAttentionRevision,
};
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalPlace};
fn terminal() -> TerminalHandle {
    let info = TerminalInfo::new(
        vessel_proto::TerminalId::random(),
        TerminalPlace::Session {
            session: vessel_proto::SessionId::random(),
        },
        std::path::PathBuf::from("/synthetic"),
        TerminalKind::Shell {
            program: std::path::PathBuf::from("/synthetic-shell"),
        },
        chrono::DateTime::UNIX_EPOCH,
    );
    TerminalHandle(Arc::new(Shared {
        info: tokio::sync::watch::Sender::new(info),
        attention_claim: Mutex::new(None),
        process: Mutex::new(None),
        ring: Mutex::new(Ring::bounded(
            super::Scrollback::DEFAULT.records,
            super::Scrollback::DEFAULT.bytes,
            vessel_proto::EventSeq::ZERO,
        )),
        input: None,
        kill: tokio_util::sync::CancellationToken::new(),
        restarts: None,
        pid: AtomicU32::new(0),
    }))
}
#[test]
fn stale_view_and_producer_cannot_consume_or_finish_newer_work() {
    let handle = terminal();
    let working = handle
        .report_activity(
            TerminalAttentionRevision::default(),
            TerminalActivitySignal::Working,
        )
        .unwrap();
    let finished = handle
        .report_activity(working.revision, TerminalActivitySignal::Idle)
        .unwrap();
    let newer = handle
        .report_activity(finished.revision, TerminalActivitySignal::Working)
        .unwrap();
    assert_eq!(
        handle.clear_attention(finished.revision),
        Err(super::AttentionError::Stale)
    );
    assert_eq!(
        handle.report_activity(finished.revision, TerminalActivitySignal::Idle),
        Err(super::AttentionError::Stale)
    );
    assert_eq!(handle.info().attention, Some(newer));
    assert_eq!(
        handle.claim_attention(finished.revision),
        NotificationClaim::NoAttention
    );
}
#[test]
fn concurrent_terminal_claims_admit_one_revision_once() {
    let handle = terminal();
    let needs = handle
        .report_activity(
            TerminalAttentionRevision::default(),
            TerminalActivitySignal::NeedsInput,
        )
        .unwrap();
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| handle.claim_attention(needs.revision));
        let b = scope.spawn(|| handle.claim_attention(needs.revision));
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == NotificationClaim::Admitted)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == NotificationClaim::AlreadyClaimed)
            .count(),
        1
    );
    let _cleared = handle.clear_attention(needs.revision).unwrap();
    assert_eq!(
        handle.claim_attention(needs.revision),
        NotificationClaim::NoAttention
    );
}
#[test]
fn activity_cannot_restart_an_ended_process() {
    let handle = terminal();
    handle
        .0
        .info
        .send_modify(|info| info.status = vessel_proto::SessionStatus::Exited { code: Some(0) });
    assert_eq!(
        handle.report_activity(
            TerminalAttentionRevision::default(),
            TerminalActivitySignal::Working
        ),
        Err(super::AttentionError::Ended)
    );
}
