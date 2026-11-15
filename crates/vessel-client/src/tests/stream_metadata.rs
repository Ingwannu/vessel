//! A valid subscription frame must describe the log actually requested.

use super::*;
use crate::StreamError;
use vessel_proto::TerminalId;
use vessel_proto::terminals::{
    SubscribeTerminal, TerminalInfo, TerminalKind, TerminalPlace, TerminalSubscribed,
};

pub(super) fn other_session() -> SessionId {
    "00000000-0000-4000-8000-000000000002".parse().unwrap()
}

fn terminal_id() -> TerminalId {
    "00000000-0000-4000-8000-000000000001".parse().unwrap()
}

impl FakeNode {
    pub(super) async fn metadata_terminal_subscription(
        &self,
        context: &Context,
        request: SubscribeTerminal,
    ) -> Result<TerminalSubscribed, WireError> {
        let ordinal = self.subscribes.fetch_add(1, Ordering::Relaxed);
        let sink = context
            .outbound
            .open_subscription(request.from)
            .await
            .map_err(OutboundError::to_wire)?;
        let subscription = sink.id();
        drop(tokio::spawn(async move {
            sink.end(SubscriptionEnd::SessionEnded).await;
        }));
        let id = if self.mismatch_after.is_some_and(|after| ordinal >= after) {
            "00000000-0000-4000-8000-000000000002".parse().unwrap()
        } else {
            request.terminal
        };
        let terminal = TerminalInfo::new(
            id,
            TerminalPlace::Session {
                session: session_id(),
            },
            PathBuf::from("/fixture"),
            TerminalKind::Shell {
                program: PathBuf::from("fixture"),
            },
            DateTime::UNIX_EPOCH,
        );
        Ok(TerminalSubscribed {
            subscription,
            terminal,
        })
    }
}

#[tokio::test]
async fn matching_session_metadata_opens_the_requested_log() {
    let client = connect(Arc::new(FakeNode::new(0))).await;
    let mut stream = client.follow(session_id(), EventSeq::ZERO).await.unwrap();
    assert_eq!(stream.info().id, session_id());
    assert_eq!(stream.next().await.unwrap().unwrap().seq, EventSeq::ZERO);
}

#[tokio::test]
async fn wrong_session_metadata_never_exposes_a_stream_or_records() {
    let mut node = FakeNode::new(0);
    node.mismatch_after = Some(0);
    let client = connect(Arc::new(node)).await;
    assert_eq!(
        client
            .follow(session_id(), EventSeq::ZERO)
            .await
            .unwrap_err(),
        StreamError::UnverifiedIdentity
    );
    // A metadata mismatch ends this follow, not the authenticated connection.
    assert_eq!(client.session(session_id()).await.unwrap().id, session_id());
}

#[tokio::test]
async fn wrong_resubscription_metadata_stops_before_its_records() {
    let mut node = FakeNode::new(0);
    node.mismatch_after = Some(1);
    let node = Arc::new(node);
    let client = connect(Arc::clone(&node)).await;
    let mut stream = client.follow(session_id(), EventSeq::ZERO).await.unwrap();
    for expected in 0..STORED {
        assert_eq!(
            stream.next().await.unwrap().unwrap().seq,
            EventSeq::new(expected)
        );
    }
    assert_eq!(
        stream.next().await,
        Some(Err(StreamError::UnverifiedIdentity))
    );
    assert!(stream.next().await.is_none());
    assert_eq!(stream.cursor(), EventSeq::new(STORED));
    assert_eq!(node.subscribes.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn matching_terminal_metadata_opens_and_ends_normally() {
    let client = connect(Arc::new(FakeNode::new(0))).await;
    let mut stream = client
        .follow_terminal(terminal_id(), EventSeq::ZERO)
        .await
        .unwrap();
    assert_eq!(stream.info().id, terminal_id());
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn wrong_terminal_metadata_never_exposes_a_stream() {
    let mut node = FakeNode::new(0);
    node.mismatch_after = Some(0);
    let client = connect(Arc::new(node)).await;
    let error = client
        .follow_terminal(terminal_id(), EventSeq::ZERO)
        .await
        .unwrap_err();
    assert_eq!(error, StreamError::UnverifiedIdentity);
    assert!(error.is_fatal());
    assert_eq!(client.session(session_id()).await.unwrap().id, session_id());
}
