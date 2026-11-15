//! A stalled failure notice shares the close owner's finite lifetime.
use super::*;
use crate::automation::notices::{Notices, PER_SESSION};
use std::future::{pending, ready};
use tokio::sync::oneshot;
use vessel_proto::{Ack, SessionId};

const TEST_DEADLINE: Duration = Duration::from_secs(15);

#[tokio::test(start_paused = true)]
async fn held_failure_notice_releases_following_without_claiming_close() {
    tokio::time::timeout(TEST_DEADLINE, async {
        let notices = Arc::new(Notices::default());
        let parent = SessionId::random();
        let mut slots: Vec<_> = (0..PER_SESSION)
            .map(|_slot| notices.follow(parent).unwrap())
            .collect();
        let following = slots.pop().unwrap();
        let (sending, entered) = oneshot::channel();
        let (closing, closed) = oneshot::channel();
        let tasks = TaskTracker::new();
        let worker = tasks.spawn(async move {
            let _following = following;
            supervision::run(
                ready(Err(WireError::new(
                    ErrorCode::Unavailable,
                    "PRIVATE_NOTICE_ERROR",
                ))),
                pending(),
                CancellationToken::new(),
                |end| async move {
                    let supervision::End::Followed(Err(error)) = end else {
                        panic!("wrong ending")
                    };
                    sending.send(unavailable(&error)).unwrap();
                    pending::<()>().await;
                },
                async move {
                    closing.send(()).unwrap();
                    Ok(Ack {})
                },
            )
            .await
        });
        let notice = entered.await.unwrap();
        assert!(!notice.text.contains("PRIVATE_NOTICE_ERROR"));
        assert!(notices.follow(parent).is_err());
        tokio::time::advance(supervision::CLEANUP_BUDGET).await;
        assert_eq!(worker.await.unwrap(), supervision::Cleanup::Uncertain);
        // The close future was dropped without being polled after the held
        // notice. This is settled observation, not an early empty-queue check.
        assert!(closed.await.is_err());
        let _closed = tasks.close();
        tasks.wait().await;
        let _released = notices.follow(parent).unwrap();
    })
    .await
    .unwrap();
}
