//! Request and target correlation survive multiple simultaneous card kinds.
use super::*;

fn target() -> Target {
    Target {
        machine: MachineKey::from_book("scratch"),
        session: SessionId::random(),
    }
}

#[test]
fn one_request_is_single_flight_across_all_local_seats() {
    let mut registry = Registry::default();
    let target = target();
    let request = Request::Question(QuestionId::random());
    let first = registry.begin(target.clone(), request).unwrap();
    assert_eq!(registry.begin(target.clone(), request), Err(Problem::Busy));
    let mut wrong = first.clone();
    wrong.target.machine = MachineKey::from_book("other");
    assert!(!registry.settle(&wrong));
    assert_eq!(registry.begin(target.clone(), request), Err(Problem::Busy));
    assert!(registry.settle(&first));
    let retry = registry.begin(target, request).unwrap();
    assert_ne!(retry.ticket, first.ticket);
    assert!(!registry.settle(&first));
}

#[test]
fn a_same_session_successor_and_a_permission_do_not_settle_the_old_question() {
    let mut registry = Registry::default();
    let target = target();
    let first = registry
        .begin(target.clone(), Request::Question(QuestionId::random()))
        .unwrap();
    let second = registry
        .begin(target.clone(), Request::Question(QuestionId::random()))
        .unwrap();
    let permission = registry
        .begin(target, Request::Permission(PermissionId::random()))
        .unwrap();
    let wrong = Pending {
        request: second.request,
        ..first.clone()
    };
    assert!(!registry.settle(&wrong));
    assert!(registry.settle(&permission));
    assert!(registry.settle(&second));
    assert!(registry.settle(&first));
}

#[test]
fn capacity_and_exhaustion_do_not_reuse_or_release_other_tickets() {
    let mut registry = Registry::default();
    let target = target();
    for _ in 0..MAX_PENDING {
        let _pending = registry
            .begin(target.clone(), Request::Question(QuestionId::random()))
            .unwrap();
    }
    assert_eq!(
        registry.begin(target.clone(), Request::Question(QuestionId::random())),
        Err(Problem::Busy)
    );
    let first = registry.pending.first().unwrap().clone();
    assert!(registry.settle(&first));
    registry.next = RequestId::new(u64::MAX);
    assert_eq!(
        registry.begin(target, Request::Question(QuestionId::random())),
        Err(Problem::Exhausted)
    );
    assert_eq!(registry.pending.len(), MAX_PENDING.saturating_sub(1));
}
