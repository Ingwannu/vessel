//! [`LogStream`]: a session's log, or a terminal's (protocol 11), as one
//! gap-free sequence.
//!
//! The stream is live (a subscription) until the subscription reports
//! `Lagged`, which means this side could not keep up. It then pages the
//! records it missed (`session.read`, `terminal.read`) at its own pace, and
//! subscribes again from its cursor once a page reports the end of the log.
//! The cursor (the next sequence number to yield) is the only state that
//! matters, so nothing is skipped and a record that arrives twice is
//! dropped.
//!
//! [Decision Log] (protocol 11, ADR 0031)
//! - Purpose: a terminal beside a session is carried exactly as a terminal
//!   session is (the same records, frames and ends), so following one is
//!   the same catch-up logic.
//! - Alternatives: a second stream type for terminals (the catch-up logic
//!   copied, and the copies drift).
//! - Choice: the stream is generic over [`Log`], which says how to
//!   subscribe to and page one kind of log; [`SessionStream`] and
//!   [`TerminalStream`] are its two instances. A terminal's scrollback
//!   drops its oldest records, which this stream reports as it reports a
//!   pruned range ([`StreamError::Missing`]).

use std::collections::VecDeque;
use std::fmt;
use std::future::Future;

use vessel_proto::terminals::{ReadTerminal, SubscribeTerminal, TerminalInfo};
use vessel_proto::{
    EventPage, EventRecord, EventSeq, ReadEvents, SessionId, SessionInfo, Subscribe,
    SubscriptionEnd, TerminalId,
};
use vessel_rpc::{CallError, Connection, Subscription, SubscriptionItem};

use crate::error::StreamError;

/// A kind of log a [`LogStream`] follows: how to subscribe to one and how
/// to page it.
pub trait Log: Send + Sync + 'static {
    /// What names one.
    type Id: Copy + fmt::Display + fmt::Debug + Send + Sync + 'static;
    /// What the subscription's reply describes it with.
    type Info: Clone + fmt::Debug + Send + Sync + 'static;

    /// Bind reply metadata to the exact requested log before any records escape.
    /// Custom logs without an identity check fail closed; known logs compare
    /// their typed IDs. This does not grant connection or mutation authority.
    #[must_use]
    fn info_matches(_id: Self::Id, _info: &Self::Info) -> bool {
        false
    }

    /// Subscribe from `from`.
    fn subscribe(
        connection: &Connection,
        id: Self::Id,
        from: EventSeq,
    ) -> impl Future<Output = Result<Subscription<Self::Info>, CallError>> + Send;

    /// At most `limit` records from `from`.
    fn read(
        connection: &Connection,
        id: Self::Id,
        from: EventSeq,
        limit: u32,
    ) -> impl Future<Output = Result<EventPage, CallError>> + Send;
}

/// A session's log (`session.subscribe`, `session.read`).
#[derive(Debug, Clone, Copy)]
pub struct SessionLog;

impl Log for SessionLog {
    type Id = SessionId;
    type Info = SessionInfo;

    fn info_matches(id: Self::Id, info: &Self::Info) -> bool {
        info.id == id
    }

    async fn subscribe(
        connection: &Connection,
        session: SessionId,
        from: EventSeq,
    ) -> Result<Subscription<SessionInfo>, CallError> {
        connection.subscribe(Subscribe { session, from }).await
    }

    async fn read(
        connection: &Connection,
        session: SessionId,
        from: EventSeq,
        limit: u32,
    ) -> Result<EventPage, CallError> {
        connection
            .call(ReadEvents {
                session,
                from,
                limit,
                wait: None,
            })
            .await
    }
}

/// A terminal's log (protocol 11: `terminal.subscribe`, `terminal.read`).
#[derive(Debug, Clone, Copy)]
pub struct TerminalLog;

impl Log for TerminalLog {
    type Id = TerminalId;
    type Info = TerminalInfo;

    fn info_matches(id: Self::Id, info: &Self::Info) -> bool {
        info.id == id
    }

    async fn subscribe(
        connection: &Connection,
        terminal: TerminalId,
        from: EventSeq,
    ) -> Result<Subscription<TerminalInfo>, CallError> {
        connection
            .subscribe_terminal(SubscribeTerminal { terminal, from })
            .await
    }

    async fn read(
        connection: &Connection,
        terminal: TerminalId,
        from: EventSeq,
        limit: u32,
    ) -> Result<EventPage, CallError> {
        connection
            .call(ReadTerminal {
                terminal,
                from,
                limit: limit.min(ReadTerminal::MAX_LIMIT),
                wait: None,
            })
            .await
    }
}

/// A session's records in order, from where [`NodeClient::follow`]
/// started, until the session ends.
///
/// [`NodeClient::follow`]: crate::NodeClient::follow
pub type SessionStream = LogStream<SessionLog>;

/// A terminal's records in order (protocol 11), from where
/// [`NodeClient::follow_terminal`] started, until the terminal ends.
///
/// [`NodeClient::follow_terminal`]: crate::NodeClient::follow_terminal
pub type TerminalStream = LogStream<TerminalLog>;

/// Records asked for per page while catching up. The node caps it further
/// by its own page limits.
const CATCH_UP_PAGE: u32 = 500;

/// Where the stream gets its next record.
///
/// The live subscription is much larger than the other states; it is not
/// boxed: one per stream, replaced in place, never queued, and the common
/// state, so boxing it would cost every stream.
#[derive(Debug)]
enum Source<I> {
    /// A live subscription.
    Live(Subscription<I>),
    /// Catching up with paged reads; `at_end` once the last page reached the
    /// end of the log.
    Paging {
        buffered: VecDeque<EventRecord>,
        at_end: bool,
    },
    /// The session ended, or a fatal error was reported.
    Done,
}

/// What to do after looking at the source.
enum Step {
    Record(EventRecord),
    CatchUp,
    ReadPage,
    Resubscribe,
    Finish,
    Fail(StreamError),
}

/// One log's records in order, from where it started, until the log ends
/// (the session's or the terminal's end).
pub struct LogStream<L: Log> {
    connection: Connection,
    id: L::Id,
    info: L::Info,
    /// The next sequence number to yield.
    cursor: EventSeq,
    source: Source<L::Info>,
    /// A record that arrived after a pruned range; yielded right after the
    /// `Missing` error that announced the range.
    held: Option<EventRecord>,
}

impl<L: Log> fmt::Debug for LogStream<L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LogStream")
            .field("id", &self.id)
            .field("cursor", &self.cursor)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl<L: Log> LogStream<L> {
    /// Follow `id` from `from` on `connection`.
    ///
    /// # Errors
    /// The first subscription failed, or its metadata identified another log.
    pub(crate) async fn open(
        connection: Connection,
        id: L::Id,
        from: EventSeq,
    ) -> Result<Self, StreamError> {
        let subscription = Self::subscribe_checked(&connection, id, from).await?;
        Ok(Self {
            connection,
            id,
            info: subscription.info().clone(),
            cursor: from,
            source: Source::Live(subscription),
            held: None,
        })
    }

    /// Every establishment, including catch-up resubscribe, has the same join.
    /// A valid frame and connection cannot bless another log's metadata.
    async fn subscribe_checked(
        connection: &Connection,
        id: L::Id,
        from: EventSeq,
    ) -> Result<Subscription<L::Info>, StreamError> {
        let subscription = L::subscribe(connection, id, from).await?;
        if !L::info_matches(id, subscription.info()) {
            return Err(StreamError::UnverifiedIdentity);
        }
        Ok(subscription)
    }

    /// What it follows, as it was when the stream opened.
    #[must_use]
    pub const fn info(&self) -> &L::Info {
        &self.info
    }

    /// The sequence number of the next record this stream will yield: where
    /// to resume with a new stream later.
    #[must_use]
    pub const fn cursor(&self) -> EventSeq {
        self.cursor
    }

    /// The next record. `None` once the log has ended (after its final
    /// status record) or after a fatal error. A
    /// [`StreamError::Missing`] is not fatal; keep calling.
    pub async fn next(&mut self) -> Option<Result<EventRecord, StreamError>> {
        if let Some(record) = self.held.take() {
            return self.accept(record);
        }
        loop {
            let step = match &mut self.source {
                Source::Done => return None,
                Source::Live(subscription) => match subscription.next().await {
                    SubscriptionItem::Record(record) => Step::Record(record),
                    SubscriptionItem::Ended(SubscriptionEnd::SessionEnded) => Step::Finish,
                    SubscriptionItem::Ended(SubscriptionEnd::Lagged { resume_from }) => {
                        tracing::debug!(id = %self.id, %resume_from, "fell behind; catching up");
                        Step::CatchUp
                    }
                    SubscriptionItem::Ended(SubscriptionEnd::Unsubscribed) => {
                        Step::Fail(StreamError::Unsubscribed)
                    }
                    SubscriptionItem::Disconnected => {
                        Step::Fail(StreamError::Call(CallError::Disconnected))
                    }
                },
                Source::Paging { buffered, at_end } => match buffered.pop_front() {
                    Some(record) => Step::Record(record),
                    None if *at_end => Step::Resubscribe,
                    None => Step::ReadPage,
                },
            };
            match step {
                Step::Record(record) => {
                    if let Some(item) = self.accept(record) {
                        return Some(item);
                    }
                }
                Step::CatchUp => {
                    self.source = Source::Paging {
                        buffered: VecDeque::new(),
                        at_end: false,
                    };
                }
                Step::ReadPage => {
                    if let Err(error) = self.read_page().await {
                        return Some(Err(self.fail(error)));
                    }
                }
                Step::Resubscribe => {
                    match Self::subscribe_checked(&self.connection, self.id, self.cursor).await {
                        Ok(subscription) => self.source = Source::Live(subscription),
                        Err(error) => return Some(Err(self.fail(error))),
                    }
                }
                Step::Finish => {
                    self.source = Source::Done;
                    return None;
                }
                Step::Fail(error) => return Some(Err(self.fail(error))),
            }
        }
    }

    async fn read_page(&mut self) -> Result<(), StreamError> {
        let page = L::read(&self.connection, self.id, self.cursor, CATCH_UP_PAGE).await?;
        // An empty page that claims more is coming would loop forever;
        // subscribing is the right move either way.
        let at_end = page.at_end || page.records.is_empty();
        self.source = Source::Paging {
            buffered: page.records.into(),
            at_end,
        };
        Ok(())
    }

    /// Yield `record` if it is the next one; drop it if it was already
    /// yielded; report the gap if records before it are gone.
    fn accept(&mut self, record: EventRecord) -> Option<Result<EventRecord, StreamError>> {
        if record.seq < self.cursor {
            return None;
        }
        if record.seq > self.cursor {
            let missing = StreamError::Missing {
                expected: self.cursor,
                resumed_at: record.seq,
            };
            self.cursor = record.seq;
            self.held = Some(record);
            return Some(Err(missing));
        }
        match record.seq.next() {
            Some(next) => self.cursor = next,
            // The log is full; nothing can follow this record.
            None => self.source = Source::Done,
        }
        Some(Ok(record))
    }

    /// End the stream after reporting `error`.
    fn fail(&mut self, error: StreamError) -> StreamError {
        self.source = Source::Done;
        error
    }
}
