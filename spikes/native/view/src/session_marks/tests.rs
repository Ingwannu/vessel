//! Local correlation and original-row provenance, with no network capability.
use super::*;
use crate::{Reach, connection::ConnectionEpochs, machine_book::Name};
use vessel_proto::{HarnessKind, NodeId, SessionInfo, SessionMode, SessionSpec};
fn fixture() -> (Machine, Card) {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    let card = Card::from_session("fixture", &info);
    let context = ConnectionEpochs::default()
        .admit(Name::parse("fixture").unwrap(), NodeId::random())
        .unwrap();
    let machine = Machine {
        name: "fixture".to_owned(),
        node: Some(context.node()),
        connection_epoch: Some(context.epoch()),
        reach: Reach::Online,
        sessions: vec![card.clone()],
        agents: vec![],
        protocol: None,
        version: None,
    };
    (machine, card)
}
#[test]
fn missing_admission_or_foreign_row_cannot_capture_intent() {
    let (mut machine, card) = fixture();
    let target = CapturedSessionTarget::capture(&machine, &card).unwrap();
    assert_eq!(target.context(), &machine.connection_context().unwrap());
    machine.connection_epoch = None;
    assert_eq!(
        CapturedSessionTarget::capture(&machine, &card),
        Err(CaptureError::IdentityUnavailable)
    );
    let (mut machine, mut card) = fixture();
    card.session = SessionId::random();
    assert_eq!(
        CapturedSessionTarget::capture(&machine, &card),
        Err(CaptureError::ForeignRow)
    );
    machine.sessions.clear();
    assert_eq!(
        CapturedSessionTarget::capture(&machine, &card),
        Err(CaptureError::ForeignRow)
    );
}
#[test]
fn stale_or_duplicate_reply_cannot_consume_a_successor_request() {
    let (machine, card) = fixture();
    let target = CapturedSessionTarget::capture(&machine, &card).unwrap();
    let mut requests = Requests::default();
    let first = requests.begin(target.clone(), Mark::Pin).unwrap();
    assert_eq!(
        requests.begin(target.clone(), Mark::Unpin),
        Err(BeginError::Busy)
    );
    let mut altered = first.clone();
    altered.mark = Mark::Unpin;
    assert!(!requests.finish(&altered));
    assert!(requests.finish(&first));
    let second = requests.begin(target, Mark::Unpin).unwrap();
    assert!(!requests.finish(&first));
    assert!(requests.is_pending());
    assert!(requests.finish(&second));
    assert!(!requests.finish(&second));
}
#[test]
fn read_boundary_and_ticket_exhaustion_refuse_without_pending_work() {
    let (machine, card) = fixture();
    let mut target = CapturedSessionTarget::capture(&machine, &card).unwrap();
    let through = EventSeq::new(7);
    target.read_through = None;
    let mut requests = Requests::default();
    assert_eq!(
        requests.begin(target.clone(), Mark::Read { through }),
        Err(BeginError::ReadBoundary)
    );
    target.read_through = Some(through);
    let ask = requests
        .begin(target.clone(), Mark::Read { through })
        .unwrap();
    assert!(requests.finish(&ask));
    requests.issued = u64::MAX;
    assert_eq!(
        requests.begin(target, Mark::Pin),
        Err(BeginError::Exhausted)
    );
    assert!(!requests.is_pending());
}
