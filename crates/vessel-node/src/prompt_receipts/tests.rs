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
