//! A session's parking after its records (ADR 0038; row 334's order for a
//! parked reload, ADR 0110).

use vessel_proto::parking::{ParkReason, Parked};
use vessel_proto::{EventRecord, EventSeq, HarnessSessionId, SessionEvent};

use super::parked_after;

fn record(seq: u64, event: SessionEvent) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: chrono::DateTime::UNIX_EPOCH,
        event,
    }
}

fn reloaded() -> SessionEvent {
    SessionEvent::Reloaded {
        harness_session: HarnessSessionId::new("conversation"),
        appended: 0,
    }
}

#[test]
fn a_parked_reload_leaves_the_session_unparked() {
    let parked = Some(Parked {
        since: chrono::DateTime::UNIX_EPOCH,
        reason: ParkReason::Idle,
    });
    let records = [record(5, SessionEvent::Woken), record(6, reloaded())];
    assert_eq!(parked_after(parked, &records), None);
}

#[test]
fn parked_then_woken_in_order() {
    let parked = record(
        2,
        SessionEvent::Parked {
            reason: ParkReason::Asked,
        },
    );
    assert_eq!(
        parked_after(None, std::slice::from_ref(&parked)),
        Some(Parked {
            since: chrono::DateTime::UNIX_EPOCH,
            reason: ParkReason::Asked,
        })
    );
    assert_eq!(
        parked_after(
            None,
            &[
                parked,
                record(3, SessionEvent::Woken),
                record(4, reloaded())
            ]
        ),
        None
    );
}
