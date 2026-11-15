//! Actual supervision and admission with an authenticated-peer close future held.
use super::*;
use crate::automation::notices::{Notices, PER_SESSION};
use crate::prompt_receipts::PromptReceipts;
use crate::transfers::Holder;
use std::future::{pending, ready};
use std::sync::{Mutex, PoisonError};
use tokio::sync::oneshot;
use vessel_proto::{Ack, SessionId};

const TEST_DEADLINE: Duration = Duration::from_secs(15);
const CANARY: &str = "PRIVATE_REMOTE_FOLLOW_MESSAGE";

#[derive(Clone, Copy)]
enum Case {
    Finished,
    ParentEnded,
    Stopped,
}

#[cfg(test)]
async fn held_close(case: Case) {
    let notices = Arc::new(Notices::default());
    let parent = SessionId::random();
    let mut slots: Vec<_> = (0..PER_SESSION)
        .map(|_slot| notices.follow(parent).unwrap())
        .collect();
    let following = slots.pop().unwrap();
    let _unrelated_following = notices.follow(SessionId::random()).unwrap();
    let receipts = Arc::new(PromptReceipts::default());
    let holder = Holder::Agent {
        node: NodeId::random(),
        session: parent,
    };
    let session = SessionId::random();
    let target = receipts.reserve(holder, session).unwrap();
    let unrelated = receipts.reserve(holder, session).unwrap();
    let request = ClosePromptReceipt {
        session,
        receipt: target.id,
    };
    let stop = CancellationToken::new();
    if matches!(case, Case::Stopped) {
        stop.cancel();
    }
    let (closing, polled) = oneshot::channel();
    let (reported, report) = oneshot::channel();
    let tasks = TaskTracker::new();
    let worker = tasks.spawn(async move {
        let _following = following;
        supervision::run(
            async move {
                if matches!(case, Case::Finished) {
                    Ok(())
                } else {
                    pending().await
                }
            },
            async move {
                if !matches!(case, Case::ParentEnded) {
                    pending::<()>().await;
                }
            },
            stop,
            |end| async move {
                assert!(matches!(
                    (case, end),
                    (Case::Finished, supervision::End::Followed(Ok(())))
                        | (Case::ParentEnded, supervision::End::ParentEnded)
                        | (Case::Stopped, supervision::End::Stopped)
                ));
                reported.send(()).unwrap();
            },
            async move {
                closing.send(request).unwrap();
                pending::<Result<Ack, WireError>>().await
            },
        )
        .await
    });
    let close = polled.await.unwrap();
    report.await.unwrap();
    assert_eq!(close.session, session);
    assert_eq!(close.receipt, target.id);
    assert!(
        notices.follow(parent).is_err(),
        "cleanup still holds the final slot"
    );
    tokio::time::advance(supervision::CLEANUP_BUDGET).await;
    assert_eq!(worker.await.unwrap(), supervision::Cleanup::Uncertain);
    let _closed = tasks.close();
    tasks.wait().await;
    let _released = notices.follow(parent).unwrap();
    // Uncertain cleanup is not remote deletion, and never touches a neighbour.
    receipts.close(holder, session, unrelated.id).unwrap();
    receipts.close(holder, session, target.id).unwrap();
}

#[tokio::test(start_paused = true)]
async fn final_notice_parent_end_and_stop_release_capacity_despite_a_held_close() {
    for case in [Case::Finished, Case::ParentEnded, Case::Stopped] {
        tokio::time::timeout(TEST_DEADLINE, held_close(case))
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn a_refused_close_retains_only_its_closed_code() {
    let result = supervision::run(
        ready(Ok(())),
        pending(),
        CancellationToken::new(),
        |_end| ready(()),
        ready(Err(WireError::new(ErrorCode::Forbidden, CANARY))),
    )
    .await;
    assert_eq!(result, supervision::Cleanup::Refused(ErrorCode::Forbidden));
    assert!(!format!("{result:?}").contains(CANARY));
}

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn actual_follower_warning_and_parent_notice_never_format_peer_text() {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let writer = Capture(Arc::clone(&bytes));
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(move || writer.clone())
        .finish();
    let notice = tracing::subscriber::with_default(subscriber, || {
        unavailable(&WireError::new(ErrorCode::Forbidden, CANARY))
    });
    let captured = bytes.lock().unwrap_or_else(PoisonError::into_inner);
    let log = String::from_utf8_lossy(&captured);
    assert!(log.contains("Forbidden"));
    assert!(log.contains("remote turn notification could not be followed"));
    assert!(!log.contains(CANARY));
    assert!(notice.last);
    assert!(notice.text.contains("became unavailable"));
    assert!(!notice.text.contains(CANARY));
}
