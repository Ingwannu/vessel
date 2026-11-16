//! The two additive SDK corrections join at the actual lag-resubscribe boundary.
use super::stream_metadata::{phase, within};
use super::*;
use crate::{LogEnd, StreamError};

#[tokio::test]
async fn wrong_resubscription_identity_is_fatal_and_never_a_normal_end() {
    within(async {
        let mut node = FakeNode::new(0);
        node.mismatch_after = Some(1);
        let node = Arc::new(node);
        let client = phase(
            "authenticate combined stream peer",
            connect(Arc::clone(&node)),
        )
        .await;
        let mut stream = phase(
            "open matched original log",
            client.follow(session_id(), EventSeq::ZERO),
        )
        .await
        .unwrap();
        assert_eq!(stream.end_reason(), None);
        for expected in 0..STORED {
            let record = phase("original records through catch-up", stream.next())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(record.seq, EventSeq::new(expected));
        }
        assert_eq!(
            phase("refuse mismatched actual resubscribe", stream.next()).await,
            Some(Err(StreamError::UnverifiedIdentity))
        );
        assert_eq!(stream.end_reason(), Some(LogEnd::FatalError));
        for _ in 0..2 {
            assert!(
                phase("fatal stream stays stopped", stream.next())
                    .await
                    .is_none()
            );
            assert_eq!(stream.end_reason(), Some(LogEnd::FatalError));
        }
        assert_eq!(stream.cursor(), EventSeq::new(STORED));
        assert_eq!(node.subscribes.load(Ordering::Relaxed), 2);
        assert_eq!(
            phase(
                "original connection still serves metadata",
                client.session(session_id())
            )
            .await
            .unwrap()
            .id,
            session_id()
        );
    })
    .await;
}
