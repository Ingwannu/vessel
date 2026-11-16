//! A valid subscription frame must describe the log actually requested.

use super::*;
use crate::StreamError;
use std::{future::Future, time::Duration};
use vessel_proto::TerminalId;
use vessel_proto::terminals::{
    SubscribeTerminal, TerminalInfo, TerminalKind, TerminalPlace, TerminalSubscribed,
};

const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE: Duration = Duration::from_secs(5);
pub(super) async fn within<T>(work: impl Future<Output = T>) -> T {
    vessel_testkit::within(SAFETY_NET, work)
        .await
        .expect("metadata test safety net")
}
pub(super) async fn phase<T>(name: &'static str, work: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, work)
        .await
        .unwrap_or_else(|_elapsed| panic!("stream metadata phase timed out: {name}"))
}

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
    within(async {
        let client = phase(
            "authenticate memory peer",
            connect(Arc::new(FakeNode::new(0))),
        )
        .await;
        let mut stream = phase(
            "matched session subscription",
            client.follow(session_id(), EventSeq::ZERO),
        )
        .await
        .unwrap();
        assert_eq!(stream.info().id, session_id());
        assert_eq!(
            phase("original first record", stream.next())
                .await
                .unwrap()
                .unwrap()
                .seq,
            EventSeq::ZERO
        );
    })
    .await;
}

#[tokio::test]
async fn wrong_session_metadata_never_exposes_a_stream_or_records() {
    within(async {
        let mut node = FakeNode::new(0);
        node.mismatch_after = Some(0);
        let client = phase("authenticate wrong-metadata peer", connect(Arc::new(node))).await;
        assert_eq!(
            phase(
                "wrong session subscription refusal",
                client.follow(session_id(), EventSeq::ZERO)
            )
            .await
            .unwrap_err(),
            StreamError::UnverifiedIdentity
        );
        assert_eq!(
            phase(
                "same connection after wrong metadata",
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

#[tokio::test]
async fn wrong_resubscription_metadata_stops_before_its_records() {
    within(async {
        let mut node = FakeNode::new(0);
        node.mismatch_after = Some(1);
        let node = Arc::new(node);
        let client = phase("authenticate resubscribe peer", connect(Arc::clone(&node))).await;
        let mut stream = phase(
            "original matched subscription",
            client.follow(session_id(), EventSeq::ZERO),
        )
        .await
        .unwrap();
        for expected in 0..STORED {
            assert_eq!(
                phase("original records before resubscribe", stream.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .seq,
                EventSeq::new(expected)
            );
        }
        assert_eq!(
            phase("wrong metadata on resubscribe", stream.next()).await,
            Some(Err(StreamError::UnverifiedIdentity))
        );
        assert!(
            phase("no records after fatal identity refusal", stream.next())
                .await
                .is_none()
        );
        assert_eq!(stream.cursor(), EventSeq::new(STORED));
        assert_eq!(node.subscribes.load(Ordering::Relaxed), 2);
    })
    .await;
}

#[tokio::test]
async fn matching_terminal_metadata_opens_and_ends_normally() {
    within(async {
        let client = phase(
            "authenticate terminal peer",
            connect(Arc::new(FakeNode::new(0))),
        )
        .await;
        let mut stream = phase(
            "matched terminal subscription",
            client.follow_terminal(terminal_id(), EventSeq::ZERO),
        )
        .await
        .unwrap();
        assert_eq!(stream.info().id, terminal_id());
        assert!(
            phase("typed terminal stream end", stream.next())
                .await
                .is_none()
        );
    })
    .await;
}

#[tokio::test]
async fn wrong_terminal_metadata_never_exposes_a_stream() {
    within(async {
        let mut node = FakeNode::new(0);
        node.mismatch_after = Some(0);
        let client = phase("authenticate wrong terminal peer", connect(Arc::new(node))).await;
        let error = phase(
            "wrong terminal subscription refusal",
            client.follow_terminal(terminal_id(), EventSeq::ZERO),
        )
        .await
        .unwrap_err();
        assert_eq!(error, StreamError::UnverifiedIdentity);
        assert!(error.is_fatal());
        assert_eq!(
            phase(
                "same connection after wrong terminal",
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
