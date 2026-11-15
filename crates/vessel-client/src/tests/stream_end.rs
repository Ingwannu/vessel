//! Actual framed ends distinguish normal completion from fatal/sequence stopping.
use super::*;
use crate::{LogEnd, StreamError};
use std::future::Future;
use std::time::Duration;
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Normal,
    Fatal,
    Exhausted,
}
#[derive(Clone)]
pub(super) struct Probe {
    mode: Mode,
    release: Arc<Notify>,
    done: Arc<Notify>,
}
impl Probe {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            release: Arc::new(Notify::new()),
            done: Arc::new(Notify::new()),
        }
    }
}

pub(super) async fn subscribe(
    context: &Context,
    request: Subscribe,
    probe: Probe,
) -> Result<Subscribed, WireError> {
    let mut sink = context
        .outbound
        .open_subscription(request.from)
        .await
        .map_err(OutboundError::to_wire)?;
    let subscription = sink.id();
    drop(tokio::spawn(async move {
        sink.send(Arc::new(record(request.from.get())))
            .await
            .unwrap();
        match probe.mode {
            Mode::Normal => sink.end(SubscriptionEnd::SessionEnded).await,
            Mode::Fatal => sink.end(SubscriptionEnd::Unsubscribed).await,
            Mode::Exhausted => {
                // Keep the actual subscription open: MAX is not a peer End.
                probe.release.notified().await;
                drop(sink);
            }
        }
        probe.done.notify_one();
    }));
    Ok(Subscribed {
        subscription,
        info: info(),
    })
}

async fn bounded<T>(work: impl Future<Output = T>) -> T {
    tokio::time::timeout(DEADLINE, work).await.unwrap()
}
async fn open_probe(mode: Mode, from: EventSeq) -> (crate::SessionStream, Probe) {
    let probe = Probe::new(mode);
    let mut node = FakeNode::new(0);
    node.end_probe = Some(probe.clone());
    let client = connect(Arc::new(node)).await;
    (client.follow(session_id(), from).await.unwrap(), probe)
}

#[tokio::test]
async fn only_the_ordered_typed_end_reports_session_ended() {
    bounded(async {
        let (mut stream, probe) = open_probe(Mode::Normal, EventSeq::ZERO).await;
        assert_eq!(stream.end_reason(), None);
        assert_eq!(stream.next().await.unwrap().unwrap().seq, EventSeq::ZERO);
        assert_eq!(stream.end_reason(), None);
        assert!(stream.next().await.is_none());
        assert_eq!(stream.end_reason(), Some(LogEnd::SessionEnded));
        assert!(stream.next().await.is_none());
        assert_eq!(stream.end_reason(), Some(LogEnd::SessionEnded));
        probe.done.notified().await;
    })
    .await;
}

#[tokio::test]
async fn fatal_error_keeps_its_original_error_and_never_becomes_normal_end() {
    bounded(async {
        let (mut stream, probe) = open_probe(Mode::Fatal, EventSeq::ZERO).await;
        assert!(stream.next().await.unwrap().is_ok());
        assert_eq!(stream.end_reason(), None);
        assert_eq!(stream.next().await, Some(Err(StreamError::Unsubscribed)));
        assert_eq!(stream.end_reason(), Some(LogEnd::FatalError));
        assert!(stream.next().await.is_none());
        assert_eq!(stream.end_reason(), Some(LogEnd::FatalError));
        probe.done.notified().await;
    })
    .await;
}

#[tokio::test]
async fn final_sequence_without_any_end_frame_reports_only_exhaustion() {
    bounded(async {
        let last = EventSeq::new(u64::MAX);
        let (mut stream, probe) = open_probe(Mode::Exhausted, last).await;
        assert_eq!(stream.end_reason(), None);
        assert_eq!(stream.next().await.unwrap().unwrap().seq, last);
        assert_eq!(stream.end_reason(), Some(LogEnd::SequenceExhausted));
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
        assert_eq!(stream.end_reason(), Some(LogEnd::SequenceExhausted));
        probe.release.notify_one();
        probe.done.notified().await;
    })
    .await;
}

#[tokio::test]
async fn a_missing_range_preserves_receiving_and_its_held_record() {
    bounded(async {
        let client = connect(Arc::new(FakeNode::new(3))).await;
        let mut stream = client.follow(session_id(), EventSeq::ZERO).await.unwrap();
        assert_eq!(
            stream.next().await,
            Some(Err(StreamError::Missing {
                expected: EventSeq::ZERO,
                resumed_at: EventSeq::new(3),
            }))
        );
        assert_eq!(stream.end_reason(), None);
        assert_eq!(stream.next().await.unwrap().unwrap().seq, EventSeq::new(3));
        assert_eq!(stream.end_reason(), None);
        while let Some(record) = stream.next().await {
            assert!(record.is_ok());
        }
        assert_eq!(stream.end_reason(), Some(LogEnd::SessionEnded));
    })
    .await;
}
