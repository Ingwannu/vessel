//! One live subscription on the client side.

use tokio::sync::{mpsc, oneshot};
use vessel_proto::terminals::TerminalInfo;
use vessel_proto::{EventRecord, SessionInfo, SubscriptionEnd, SubscriptionId};

use super::driver::{Command, Opened, Subject};

/// What [`Subscription::next`] yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionItem {
    /// The next record, in sequence order.
    Record(EventRecord),
    /// The subscription ended; no more records follow. On
    /// `Lagged{resume_from}` nothing was skipped: read or resubscribe from
    /// `resume_from`.
    Ended(SubscriptionEnd),
    /// The connection closed; records after the last one yielded may exist
    /// on the node.
    Disconnected,
}

/// How a subscription finished, remembered so `next` keeps answering.
#[derive(Debug, Clone, Copy)]
enum Finished {
    Ended(SubscriptionEnd),
    Disconnected,
}

/// A session event stream opened by [`Connection::subscribe`], or (with
/// `I` a [`TerminalInfo`]) a terminal's opened by
/// [`Connection::subscribe_terminal`] (protocol 11): the same records and
/// ends either way. Dropping it unsubscribes on the node.
///
/// [`Connection::subscribe`]: super::Connection::subscribe
/// [`Connection::subscribe_terminal`]: super::Connection::subscribe_terminal
#[derive(Debug)]
pub struct Subscription<I = SessionInfo> {
    id: SubscriptionId,
    info: I,
    records: mpsc::Receiver<EventRecord>,
    end: oneshot::Receiver<SubscriptionEnd>,
    finished: Option<Finished>,
    commands: mpsc::Sender<Command>,
}

/// A terminal's records (protocol 11, ADR 0031).
pub type TerminalSubscription = Subscription<TerminalInfo>;

impl<I> Subscription<I> {
    /// The subscription `opened` describes, if it follows what `pick`
    /// takes; otherwise it is dropped (and so unsubscribed) and `None`.
    pub(super) fn new(
        opened: Opened,
        commands: mpsc::Sender<Command>,
        pick: impl FnOnce(Subject) -> Option<I>,
    ) -> Option<Self> {
        let Opened {
            subscription,
            subject,
            records,
            end,
        } = opened;
        let Some(info) = pick(subject) else {
            if commands
                .try_send(Command::Unsubscribe(subscription))
                .is_err()
            {
                tracing::trace!(%subscription, "could not queue unsubscribe");
            }
            return None;
        };
        Some(Self {
            id: subscription,
            info,
            records,
            end,
            finished: None,
            commands,
        })
    }

    /// The node's id for this subscription.
    #[must_use]
    pub const fn id(&self) -> SubscriptionId {
        self.id
    }

    /// What it follows, as it was when the subscription opened.
    #[must_use]
    pub const fn info(&self) -> &I {
        &self.info
    }

    /// The next record, or how the subscription ended. After the end, keeps
    /// returning the same end. Cancel-safe.
    pub async fn next(&mut self) -> SubscriptionItem {
        if let Some(finished) = self.finished {
            return finished.into();
        }
        if let Some(record) = self.records.recv().await {
            return SubscriptionItem::Record(record);
        }
        // The driver closes the record channel right after sending the end
        // reason, so the reason is available once the records are drained.
        let finished = match (&mut self.end).await {
            Ok(end) => Finished::Ended(end),
            Err(_driver_gone) => Finished::Disconnected,
        };
        self.finished = Some(finished);
        finished.into()
    }
}

impl From<Finished> for SubscriptionItem {
    fn from(finished: Finished) -> Self {
        match finished {
            Finished::Ended(end) => Self::Ended(end),
            Finished::Disconnected => Self::Disconnected,
        }
    }
}

impl<I> Drop for Subscription<I> {
    fn drop(&mut self) {
        if self.finished.is_some() {
            return;
        }
        // Best effort: if the queue is full, the driver notices the closed
        // record channel on the next record and unsubscribes then.
        if self
            .commands
            .try_send(Command::Unsubscribe(self.id))
            .is_err()
        {
            tracing::trace!(subscription = %self.id, "could not queue unsubscribe");
        }
    }
}
