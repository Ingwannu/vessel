//! Exercise the real writer/PendingCall drop with bounded memory sinks, no node/harness.

use super::super::{Command, DriverError, Registration, Waiter, write};
use crate::CallError;
use crate::client::{PendingCall, RequestIds};
use crate::memory::{self, Disconnected, MemoryTransport};
use futures_util::{Sink, SinkExt as _, StreamExt as _};
use std::{
    future::Future,
    num::NonZeroUsize,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::{Notify, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{ClientFrame, GetNode, Outcome, Request as _, RequestId};

const DEADLINE: Duration = Duration::from_secs(5);
async fn step<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(DEADLINE, future).await.unwrap()
}

struct HeldSink {
    inner: MemoryTransport,
    blocked: Arc<Notify>,
}
impl Sink<String> for HeldSink {
    type Error = Disconnected;
    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let result = Pin::new(&mut self.inner).poll_ready(cx);
        if result.is_pending() {
            self.blocked.notify_one();
        }
        result
    }
    fn start_send(mut self: Pin<&mut Self>, item: String) -> Result<(), Self::Error> {
        Pin::new(&mut self.inner).start_send(item)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}
struct Fixture {
    commands: mpsc::Sender<Command>,
    registrations: mpsc::Receiver<Registration>,
    peer: MemoryTransport,
    blocked: Arc<Notify>,
    closed: CancellationToken,
    task: JoinHandle<Result<(), DriverError<Disconnected>>>,
}
impl Fixture {
    async fn held() -> Self {
        let (mut inner, peer) = memory::pair(NonZeroUsize::new(1).unwrap());
        // Fill the transport before the writer starts: its first admission waits.
        inner.send("held transport".to_owned()).await.unwrap();
        let blocked = Arc::new(Notify::new());
        let sink = HeldSink {
            inner,
            blocked: Arc::clone(&blocked),
        };
        let (commands, receiver) = mpsc::channel(2);
        let (register, registrations) = mpsc::channel(64);
        let closed = CancellationToken::new();
        let task = tokio::spawn(write(
            sink,
            receiver,
            register,
            Arc::new(RequestIds::default()),
            closed.clone(),
        ));
        Self {
            commands,
            registrations,
            peer,
            blocked,
            closed,
            task,
        }
    }
    async fn frame(&mut self) -> ClientFrame {
        let text = step(self.peer.next()).await.unwrap().unwrap();
        serde_json::from_str(&text).unwrap()
    }
    async fn release(&mut self) {
        assert_eq!(
            step(self.peer.next()).await.unwrap().unwrap(),
            "held transport"
        );
    }
    async fn finish(mut self) {
        self.closed.cancel();
        step(&mut self.task).await.unwrap().unwrap();
    }
}
fn queued(
    commands: &mpsc::Sender<Command>,
    id: RequestId,
) -> impl Future<Output = Result<Outcome, CallError>> + '_ {
    let (response, answer) = oneshot::channel();
    commands
        .try_send(Command::call(
            id,
            GetNode {}.into_call(),
            Waiter::Unary(response),
        ))
        .unwrap();
    PendingCall {
        id,
        commands,
        answered: false,
    }
    .wait(answer)
}
fn request(frame: &ClientFrame) -> RequestId {
    match frame {
        ClientFrame::Request { id, .. } => *id,
        ClientFrame::Hello(_) | ClientFrame::Cancel { .. } => panic!("expected a request"),
    }
}
fn registered(fixture: &mut Fixture) -> Vec<RequestId> {
    let mut ids = Vec::new();
    while let Ok(registration) = fixture.registrations.try_recv() {
        match registration {
            Registration::Wait(id, _waiter) => ids.push(id),
            Registration::Forget(_id) => {}
        }
    }
    ids
}

#[tokio::test]
async fn a_dropped_queued_call_is_skipped_even_when_cancel_cannot_enter_the_full_queue() {
    let mut fixture = Fixture::held().await;
    let commands = fixture.commands.clone();
    let first = queued(&commands, RequestId::new(1));
    step(fixture.blocked.notified()).await;
    let discarded = queued(&commands, RequestId::new(2));
    let kept = queued(&commands, RequestId::new(3));
    assert_eq!(commands.capacity(), 0);
    // Dropping the real wait future executes PendingCall::drop. Its Cancel
    // offer fails because both command slots are occupied, independently of closure.
    drop(discarded);
    assert_eq!(commands.capacity(), 0);
    fixture.release().await;
    assert_eq!(request(&fixture.frame().await), RequestId::new(1));
    assert_eq!(request(&fixture.frame().await), RequestId::new(3));
    assert_eq!(
        registered(&mut fixture),
        [RequestId::new(1), RequestId::new(3)]
    );
    drop(first);
    drop(kept);
    fixture.finish().await;
}

#[tokio::test]
async fn a_call_dropped_while_its_sink_is_pending_never_sends_or_leaves_a_reply_registration() {
    let mut fixture = Fixture::held().await;
    let commands = fixture.commands.clone();
    let discarded = queued(&commands, RequestId::new(1));
    step(fixture.blocked.notified()).await;
    drop(discarded);
    let kept = queued(&commands, RequestId::new(2));
    fixture.release().await;
    // Cancel may still be offered, but the request it names was never committed.
    let cancel = fixture.frame().await;
    assert!(matches!(cancel, ClientFrame::Cancel { id } if id == RequestId::new(1)));
    assert_eq!(request(&fixture.frame().await), RequestId::new(2));
    assert_eq!(registered(&mut fixture), [RequestId::new(2)]);
    drop(kept);
    fixture.finish().await;
}

#[tokio::test]
async fn a_request_committed_before_caller_drop_is_not_claimed_to_be_retracted() {
    let mut fixture = Fixture::held().await;
    let commands = fixture.commands.clone();
    let committed = queued(&commands, RequestId::new(1));
    step(fixture.blocked.notified()).await;
    fixture.release().await;
    assert_eq!(request(&fixture.frame().await), RequestId::new(1));
    assert_eq!(registered(&mut fixture), [RequestId::new(1)]);
    drop(committed);
    assert!(matches!(fixture.frame().await, ClientFrame::Cancel { id } if id == RequestId::new(1)));
    fixture.finish().await;
}

#[tokio::test]
async fn both_waiter_kinds_observe_receiver_drop_without_a_command_queue() {
    let (unary, answer) = oneshot::channel();
    let mut unary = Waiter::Unary(unary);
    assert!(!unary.is_closed());
    drop(answer);
    assert!(unary.is_closed());
    step(unary.closed()).await;
    let (subscription, answer) = oneshot::channel();
    let mut subscription = Waiter::Subscribe(subscription);
    assert!(!subscription.is_closed());
    drop(answer);
    assert!(subscription.is_closed());
    step(subscription.closed()).await;
}
