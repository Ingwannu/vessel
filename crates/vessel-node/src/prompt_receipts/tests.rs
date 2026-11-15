use super::*;
use vessel_proto::NodeId;

fn holder() -> Holder {
    Holder::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    }
}

#[tokio::test]
async fn exact_turn_is_bound_to_the_opener_and_operated_session() {
    let receipts = Arc::new(PromptReceipts::default());
    let opener = holder();
    let session = SessionId::random();
    let reservation = receipts.reserve(opener, session).unwrap();
    let id = reservation.id;
    assert_eq!(
        receipts.read(holder(), session, id).await.unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(
        receipts
            .read(opener, SessionId::random(), id)
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let (started, turn) = oneshot::channel();
    reservation.follow(turn, &tasks, stop);
    started.send(EventSeq::new(74)).unwrap();
    assert_eq!(
        receipts.read(opener, session, id).await.unwrap(),
        PromptReceiptState::Started {
            turn: EventSeq::new(74)
        }
    );
    receipts.close(opener, session, id).unwrap();
    assert_eq!(
        receipts.read(opener, session, id).await.unwrap_err().code,
        ErrorCode::NotFound
    );
    let _closed = tasks.close();
    tasks.wait().await;
}

#[test]
fn failed_admission_releases_its_reservation_and_limits_are_per_holder() {
    let receipts = Arc::new(PromptReceipts::default());
    let opener = holder();
    let session = SessionId::random();
    let reservations: Vec<_> = (0..MAX_PER_HOLDER)
        .map(|_n| receipts.reserve(opener, session).unwrap())
        .collect();
    assert_eq!(
        receipts.reserve(opener, session).unwrap_err().code,
        ErrorCode::Conflict
    );
    drop(receipts.reserve(holder(), session).unwrap());
    assert_eq!(receipts.entries().len(), MAX_PER_HOLDER);
    drop(reservations);
    assert!(receipts.entries().is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_removed_prompt_is_distinct_from_a_started_turn_and_expiry_refuses_reads() {
    let receipts = Arc::new(PromptReceipts::default());
    let opener = holder();
    let session = SessionId::random();
    let reservation = receipts.reserve(opener, session).unwrap();
    let id = reservation.id;
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let (started, turn) = oneshot::channel();
    reservation.follow(turn, &tasks, stop);
    drop(started);
    assert_eq!(
        receipts.read(opener, session, id).await.unwrap(),
        PromptReceiptState::Removed
    );
    tokio::time::advance(RECEIPT_LIFETIME).await;
    assert_eq!(
        receipts.read(opener, session, id).await.unwrap_err().code,
        ErrorCode::NotFound
    );
    let _closed = tasks.close();
    tasks.wait().await;
}

#[tokio::test]
async fn relayed_receipt_identity_includes_delivering_device_and_origin() {
    use vessel_proto::{DeviceId, OriginBy};
    let receipts = Arc::new(PromptReceipts::default());
    let device = DeviceId::random();
    let node = NodeId::random();
    let origin = SessionId::random();
    let session = SessionId::random();
    let opener = Holder::Delegated {
        device,
        node,
        by: OriginBy::Agent {
            session: origin,
            depth: 1,
        },
    };
    let reservation = receipts.reserve(opener, session).unwrap();
    let id = reservation.id;
    for impostor in [
        Holder::Delegated {
            device: DeviceId::random(),
            node,
            by: OriginBy::Agent {
                session: origin,
                depth: 1,
            },
        },
        Holder::Delegated {
            device,
            node: NodeId::random(),
            by: OriginBy::Agent {
                session: origin,
                depth: 1,
            },
        },
        Holder::Delegated {
            device,
            node,
            by: OriginBy::Agent {
                session: SessionId::random(),
                depth: 1,
            },
        },
        Holder::Delegated {
            device,
            node,
            by: OriginBy::Owner,
        },
        Holder::Agent {
            node,
            session: origin,
        },
    ] {
        assert_eq!(
            receipts.read(impostor, session, id).await.unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            receipts.close(impostor, session, id).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
    receipts.close(opener, session, id).unwrap();
    assert!(receipts.entries().is_empty());
}

#[tokio::test(start_paused = true)]
async fn cancelled_long_poll_releases_reader_budget_and_expiry_reclaims_admission() {
    use std::future::Future as _;
    use std::task::{Context, Poll, Waker};
    let receipts = Arc::new(PromptReceipts::default());
    let opener = holder();
    let session = SessionId::random();
    let reservation = receipts.reserve(opener, session).unwrap();
    let mut reading = Box::pin(receipts.read(opener, session, reservation.id));
    // Poll once to reach the long-poll await without a timer or scheduler race.
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(reading.as_mut().poll(&mut context), Poll::Pending));
    assert_eq!(receipts.readers.available_permits(), MAX_READERS - 1);
    drop(reading);
    assert_eq!(receipts.readers.available_permits(), MAX_READERS);
    let retained: Vec<_> = (1..MAX_PER_HOLDER)
        .map(|_n| receipts.reserve(opener, session).unwrap())
        .collect();
    assert_eq!(
        receipts.reserve(opener, session).unwrap_err().code,
        ErrorCode::Conflict
    );
    tokio::time::advance(RECEIPT_LIFETIME).await;
    let fresh = receipts.reserve(opener, session).unwrap();
    assert_eq!(receipts.entries().len(), 1);
    drop(fresh);
    drop(retained);
    drop(reservation);
    assert!(receipts.entries().is_empty());
}
