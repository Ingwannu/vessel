//! Request and target correlation survive multiple simultaneous card kinds.
use super::*;
use crate::{connection::ConnectionEpochs, devices::MachineKey};
use vessel_proto::NodeId;

fn target() -> Target {
    Target {
        connection: ConnectionEpochs::default()
            .admit(MachineKey::from_book("scratch"), NodeId::random())
            .unwrap(),
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
    wrong.target.connection = ConnectionEpochs::default()
        .admit(MachineKey::from_book("other"), target.connection.node())
        .unwrap();
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

#[test]
fn reconnects_and_aliases_cannot_double_submit_the_same_node_request() {
    let mut registry = Registry::default();
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let first = Target {
        connection: epochs
            .admit(MachineKey::from_book("scratch"), node)
            .unwrap(),
        session: SessionId::random(),
    };
    let request = Request::Permission(PermissionId::random());
    let pending = registry.begin(first.clone(), request).unwrap();
    for connection in [
        epochs
            .admit(MachineKey::from_book("scratch"), node)
            .unwrap(),
        epochs
            .admit(MachineKey::from_book("other-alias"), node)
            .unwrap(),
    ] {
        assert_ne!(connection.epoch(), first.connection.epoch());
        let changed = Target {
            connection,
            session: first.session,
        };
        assert_eq!(registry.begin(changed.clone(), request), Err(Problem::Busy));
        let wrong = Pending {
            target: changed,
            ..pending.clone()
        };
        assert!(!registry.settle(&wrong));
    }
    assert!(registry.settle(&pending));
    let retry = registry
        .begin(
            Target {
                connection: epochs
                    .admit(MachineKey::from_book("scratch"), node)
                    .unwrap(),
                session: first.session,
            },
            request,
        )
        .unwrap();
    assert_ne!(retry.ticket, pending.ticket);
    assert!(!registry.settle(&pending));
    assert!(registry.settle(&retry));
}
