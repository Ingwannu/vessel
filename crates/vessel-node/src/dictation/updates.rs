//! Lossless bounded update delivery, without provider or transport policy.
//!
//! Failure stays generic so the actor can retain its typed internal cause until
//! the RPC boundary. Sending owns the response-ready point: a canceled receiver
//! returns its exact update before any await, including a terminal result.

use std::collections::VecDeque;
use tokio::sync::oneshot;
use vessel_proto::dictation::Transcript;

const MAX_PARTIALS: usize = 2;

/// Retain a terminal separately from bounded partial backpressure.
pub(super) enum End<F> {
    Final(Transcript),
    Failed(F),
}
/// Keep partial and terminal ownership distinct at response readiness.
pub(super) enum Event<F> {
    Partial(Transcript),
    End(End<F>),
}
/// Refuse extra waits without consuming the pending update.
pub(super) enum Reply<F> {
    Update(Event<F>),
    Refused(ReadRefusal),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
/// Distinguish a conflicting wait from an absent stream.
pub(super) enum ReadRefusal {
    #[error("a dictation update wait is already pending")]
    Waiting,
    #[error("the dictation stream no longer exists")]
    Gone,
}
#[derive(Debug, thiserror::Error)]
/// Return the owned transcript when the bounded buffer cannot accept it.
pub(super) enum PartialRefusal {
    #[error("the dictation partial buffer holds {MAX_PARTIALS} updates")]
    Full(Transcript),
    #[error("the dictation stream already has a terminal result")]
    Ended(Transcript),
}

#[derive(thiserror::Error)]
#[error("the dictation stream already has a terminal result")]
/// Return the later terminal without overwriting the first.
pub(super) struct EndRefusal<F>(End<F>);
impl<F> std::fmt::Debug for EndRefusal<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EndRefusal")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Open,
    Ending,
    Consumed,
}
#[derive(Debug, PartialEq, Eq)]
/// Let the actor retire only after a terminal becomes response-ready.
pub(super) enum Delivery {
    Idle,
    PartialReady,
    TerminalReady,
}

/// Restore canceled replies before another await can lose an update.
pub(super) struct Updates<F> {
    partials: VecDeque<Transcript>,
    end: Option<End<F>>,
    waiter: Option<oneshot::Sender<Reply<F>>>,
    phase: Phase,
}
impl<F> Default for Updates<F> {
    fn default() -> Self {
        Self {
            partials: VecDeque::with_capacity(MAX_PARTIALS),
            end: None,
            waiter: None,
            phase: Phase::Open,
        }
    }
}
impl<F> Updates<F> {
    /// Backpressure keeps a successful provider result outside this full queue.
    pub(super) fn can_partial(&self) -> bool {
        self.phase == Phase::Open && self.partials.len() < MAX_PARTIALS
    }
    /// No accepted audio produces a call refusal, never a fabricated terminal.
    pub(super) fn absent(&mut self) {
        self.partials.clear();
        self.end = None;
        self.phase = Phase::Consumed;
        if let Some(reply) = self.waiter.take() {
            drop(reply.send(Reply::Refused(ReadRefusal::Gone)));
        }
    }
    /// Explicit retirement discards partials but preserves the first terminal.
    pub(super) fn retire(&mut self, end: End<F>) {
        self.partials.clear();
        if self.phase == Phase::Open {
            self.end = Some(end);
            self.phase = Phase::Ending;
        }
    }

    /// Refuse overflow without dropping or coalescing transcript events.
    pub(super) fn partial(&mut self, text: Transcript) -> Result<(), PartialRefusal> {
        if self.phase != Phase::Open {
            return Err(PartialRefusal::Ended(text));
        }
        if self.partials.len() == MAX_PARTIALS {
            return Err(PartialRefusal::Full(text));
        }
        self.partials.push_back(text);
        Ok(())
    }

    /// Select one terminal independently of partial capacity.
    pub(super) fn finish(&mut self, end: End<F>) -> Result<(), EndRefusal<F>> {
        if self.phase != Phase::Open {
            return Err(EndRefusal(end));
        }
        self.end = Some(end);
        self.phase = Phase::Ending;
        Ok(())
    }

    /// Allow one pending reader and replace only a canceled one.
    pub(super) fn wait(&mut self, reply: oneshot::Sender<Reply<F>>) {
        if self.waiter.as_ref().is_some_and(oneshot::Sender::is_closed) {
            drop(self.waiter.take());
        }
        let refusal = if self.waiter.is_some() {
            Some(ReadRefusal::Waiting)
        } else if self.phase == Phase::Consumed {
            Some(ReadRefusal::Gone)
        } else {
            None
        };
        if let Some(refusal) = refusal {
            // A canceled refusal consumes no update and needs no replay record.
            drop(reply.send(Reply::Refused(refusal)));
        } else {
            self.waiter = Some(reply);
        }
    }

    /// Transfer an update only at the response-ready point.
    pub(super) fn pump(&mut self) -> Delivery {
        let Some(reply) = self.waiter.take() else {
            return Delivery::Idle;
        };
        let event = self
            .partials
            .pop_front()
            .map(Event::Partial)
            .or_else(|| self.end.take().map(Event::End));
        let Some(event) = event else {
            self.waiter = Some(reply);
            return Delivery::Idle;
        };
        let delivery = match &event {
            Event::Partial(_) => Delivery::PartialReady,
            Event::End(_) => Delivery::TerminalReady,
        };
        if let Err(returned) = reply.send(Reply::Update(event)) {
            match returned {
                Reply::Update(Event::Partial(text)) => self.partials.push_front(text),
                Reply::Update(Event::End(end)) => self.end = Some(end),
                // Only Update was sent above; keep refusals outside the buffer.
                Reply::Refused(ReadRefusal::Waiting | ReadRefusal::Gone) => {}
            }
            return Delivery::Idle;
        }
        if delivery == Delivery::TerminalReady {
            self.phase = Phase::Consumed;
        }
        delivery
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Failure {
        Upstream,
        Cancelled,
    }
    fn text(value: &str) -> Transcript {
        Transcript::try_from(value.to_owned()).unwrap()
    }
    fn pop(updates: &mut Updates<Failure>) -> Reply<Failure> {
        let (reply, mut receiver) = oneshot::channel();
        updates.wait(reply);
        assert_ne!(updates.pump(), Delivery::Idle);
        receiver.try_recv().unwrap()
    }
    fn partial(reply: Reply<Failure>) -> String {
        match reply {
            Reply::Update(Event::Partial(text)) => text.as_str().to_owned(),
            Reply::Update(Event::End(_)) | Reply::Refused(_) => panic!("expected a partial"),
        }
    }

    #[test]
    fn full_buffer_returns_ownership_and_keeps_every_partial_before_terminal() {
        let mut updates = Updates::default();
        for index in 0..MAX_PARTIALS {
            updates.partial(text(&format!("partial-{index}"))).unwrap();
        }
        let returned = match updates.partial(text("third")) {
            Err(PartialRefusal::Full(text)) => text,
            Ok(()) | Err(PartialRefusal::Ended(_)) => panic!("missing backpressure"),
        };
        assert_eq!(partial(pop(&mut updates)), "partial-0");
        updates.partial(returned).unwrap();
        updates.finish(End::Final(text("final"))).unwrap();
        for index in 1..MAX_PARTIALS {
            assert_eq!(partial(pop(&mut updates)), format!("partial-{index}"));
        }
        assert_eq!(partial(pop(&mut updates)), "third");
        match pop(&mut updates) {
            Reply::Update(Event::End(End::Final(text))) => assert!(text.as_str() == "final"),
            Reply::Update(Event::Partial(_) | Event::End(End::Failed(_))) | Reply::Refused(_) => {
                panic!("missing final")
            }
        }
        let (reply, mut receiver) = oneshot::channel();
        updates.wait(reply);
        assert!(matches!(
            receiver.try_recv().unwrap(),
            Reply::Refused(ReadRefusal::Gone)
        ));
    }

    #[test]
    fn canceled_wait_restores_the_exact_partial_and_terminal() {
        let mut updates = Updates::default();
        updates.partial(text("retained")).unwrap();
        for index in 1..MAX_PARTIALS {
            updates.partial(text(&format!("later-{index}"))).unwrap();
        }
        let (reply, receiver) = oneshot::channel();
        updates.wait(reply);
        drop(receiver);
        assert_eq!(updates.pump(), Delivery::Idle);
        assert_eq!(partial(pop(&mut updates)), "retained");
        for index in 1..MAX_PARTIALS {
            assert_eq!(partial(pop(&mut updates)), format!("later-{index}"));
        }
        updates.finish(End::Failed(Failure::Upstream)).unwrap();
        let (reply, receiver) = oneshot::channel();
        updates.wait(reply);
        drop(receiver);
        assert_eq!(updates.pump(), Delivery::Idle);
        assert!(matches!(
            pop(&mut updates),
            Reply::Update(Event::End(End::Failed(Failure::Upstream)))
        ));
    }

    #[test]
    fn an_extra_wait_is_refused_and_a_canceled_wait_can_be_replaced() {
        let mut updates = Updates::<Failure>::default();
        let (first, mut receiver) = oneshot::channel();
        updates.wait(first);
        let (second, mut refused) = oneshot::channel();
        updates.wait(second);
        assert!(matches!(
            refused.try_recv().unwrap(),
            Reply::Refused(ReadRefusal::Waiting)
        ));
        assert!(matches!(
            receiver.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        drop(receiver);
        updates.partial(text("still queued")).unwrap();
        assert_eq!(partial(pop(&mut updates)), "still queued");
    }

    #[test]
    fn a_later_terminal_cannot_relabel_an_already_selected_failure() {
        let mut updates = Updates::default();
        updates.finish(End::Failed(Failure::Upstream)).unwrap();
        let returned = updates.finish(End::Failed(Failure::Cancelled)).unwrap_err();
        assert!(matches!(returned.0, End::Failed(Failure::Cancelled)));
        let returned = updates.partial(text("too late")).unwrap_err();
        assert!(matches!(returned, PartialRefusal::Ended(_)));
        assert!(matches!(
            pop(&mut updates),
            Reply::Update(Event::End(End::Failed(Failure::Upstream)))
        ));
    }
}
