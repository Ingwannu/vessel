//! Keeping a websocket alive from inside its reader and writer: the pings
//! one end owes and the silence after which either end gives up
//! ([`Keepalive`]'s numbers).
//!
//! [Decision Log]
//! - Purpose: a peer that vanished without closing (a laptop that slept, a
//!   NAT mapping that expired, a process that cannot read any more) must
//!   not keep its connection, and what the connection holds, for good. A
//!   node's `/secure` connections had this from the start; its token
//!   `/rpc` connections had none, so a client gone that way kept its
//!   subscriptions open (found staging parking, 2026-11-14).
//! - Choice: one [`Liveness`] per socket, driven by the socket's own
//!   adapter (its `poll_next`, `poll_ready`, `poll_flush`), so the node's
//!   `/secure` carrier and its `/rpc` transport share the logic and differ
//!   only in their message types. The rules it keeps are the carrier
//!   module's: a ping goes from whichever side is polled, never from the
//!   reading side while a write is in flight.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::Sink;
use tokio::time::{Instant, Sleep};

use super::Keepalive;

/// The pings this end owes and the silence it tolerates, for one socket.
/// Without a [`Keepalive`] it does nothing.
#[derive(Debug)]
pub struct Liveness {
    idle: Option<Idle>,
    pinger: Option<Pinger>,
    /// The writing side is waiting on the socket (its last `poll_ready`,
    /// `poll_flush` or `poll_close` was pending): the reading side must
    /// not take the socket's write waker from it.
    writer_waiting: bool,
}

impl Liveness {
    /// Ping every `keepalive.ping` and give up after `keepalive.silence`
    /// without a message; `None` for neither.
    #[must_use]
    pub fn new(keepalive: Option<Keepalive>) -> Self {
        Self {
            idle: Idle::new(keepalive.map(|keepalive| keepalive.silence)),
            pinger: Pinger::new(keepalive.map(|keepalive| keepalive.ping)),
            writer_waiting: false,
        }
    }

    /// A message arrived (pings and pongs count): the silence starts again.
    pub fn heard(&mut self) {
        Idle::heard(&mut self.idle);
    }

    /// The socket has nothing to read: hand it an owed ping (unless a
    /// write is in flight) and check the silence. `Ready` with why the
    /// connection is over, `Pending` otherwise.
    pub fn quiet<S, M>(
        &mut self,
        socket: Pin<&mut S>,
        ping: impl FnOnce() -> M,
        error: impl FnOnce(S::Error) -> io::Error,
        cx: &mut Context<'_>,
    ) -> Poll<io::Error>
    where
        S: Sink<M>,
    {
        if let Err(failed) = poll_ping(&mut self.pinger, socket, ping, self.writer_waiting, cx) {
            return Poll::Ready(error(failed));
        }
        Idle::poll_expired::<()>(&mut self.idle, cx).map(|expired| match expired {
            Some(Err(silent)) => silent,
            Some(Ok(())) | None => io::Error::new(io::ErrorKind::TimedOut, "the connection ended"),
        })
    }

    /// The writing side's `poll_ready`: an owed ping goes first, then the
    /// socket must be ready again.
    ///
    /// # Errors
    /// The socket's.
    pub fn poll_ready<S, M>(
        &mut self,
        mut socket: Pin<&mut S>,
        ping: impl Fn() -> M,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), S::Error>>
    where
        S: Sink<M>,
    {
        loop {
            let ready = socket.as_mut().poll_ready(cx);
            match self.wrote(ready) {
                Poll::Ready(Ok(())) => {}
                other @ (Poll::Ready(Err(_)) | Poll::Pending) => return other,
            }
            if !ping_before_write(&mut self.pinger, socket.as_mut(), &ping)? {
                return Poll::Ready(Ok(()));
            }
        }
    }

    /// The writing side's `poll_flush` ended as `flushed`: a ping handed
    /// over before it is out with it.
    pub fn flushed<E>(&mut self, flushed: Poll<Result<(), E>>) -> Poll<Result<(), E>> {
        if let (Poll::Ready(Ok(())), Some(pinger)) = (&flushed, &mut self.pinger) {
            pinger.unflushed = false;
        }
        self.wrote(flushed)
    }

    /// Note how a writing-side call on the socket ended.
    pub fn wrote<T>(&mut self, polled: Poll<T>) -> Poll<T> {
        self.writer_waiting = polled.is_pending();
        polled
    }
}

/// When this end owes its peer a ping (a node's `/secure` connections,
/// which nobody else pings).
#[derive(Debug)]
struct Pinger {
    every: Duration,
    next: Pin<Box<Sleep>>,
    /// A ping is owed and not yet handed to the socket.
    owed: bool,
    /// A ping was handed to the socket and not yet flushed.
    unflushed: bool,
}

impl Pinger {
    fn new(every: Option<Duration>) -> Option<Self> {
        every.map(|every| Self {
            every,
            next: Box::pin(tokio::time::sleep(every)),
            owed: false,
            unflushed: false,
        })
    }

    /// Whether a ping is owed now; otherwise the next one wakes `cx`.
    fn poll_owed(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.owed && self.next.as_mut().poll(cx).is_ready() {
            self.owed = true;
            let now = Instant::now();
            self.next
                .as_mut()
                .reset(now.checked_add(self.every).unwrap_or(now));
            // Registers `cx` for the one after.
            let _pending = self.next.as_mut().poll(cx);
        }
        self.owed
    }
}

/// Hand an owed ping to `socket`, from the reading side: only when no
/// write is in flight (`writing`), and without waiting; a flush that cannot
/// finish now is finished on a later call or by the next write.
fn poll_ping<S, M>(
    pinger: &mut Option<Pinger>,
    mut socket: Pin<&mut S>,
    ping: impl FnOnce() -> M,
    writing: bool,
    cx: &mut Context<'_>,
) -> Result<(), S::Error>
where
    S: Sink<M>,
{
    let Some(pinger) = pinger else {
        return Ok(());
    };
    if writing {
        return Ok(());
    }
    if pinger.unflushed {
        match socket.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => pinger.unflushed = false,
            Poll::Ready(Err(error)) => return Err(error),
            Poll::Pending => return Ok(()),
        }
    }
    if !pinger.poll_owed(cx) {
        return Ok(());
    }
    match socket.as_mut().poll_ready(cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Ready(Err(error)) => return Err(error),
        Poll::Pending => return Ok(()),
    }
    socket.as_mut().start_send(ping())?;
    pinger.owed = false;
    match socket.poll_flush(cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Ready(Err(error)) => return Err(error),
        Poll::Pending => pinger.unflushed = true,
    }
    Ok(())
}

/// Hand an owed ping to `socket` from the writing side, before its next
/// record; `socket` is ready (its `poll_ready` returned `Ready(Ok)`).
fn ping_before_write<S, M>(
    pinger: &mut Option<Pinger>,
    socket: Pin<&mut S>,
    ping: impl FnOnce() -> M,
) -> Result<bool, S::Error>
where
    S: Sink<M>,
{
    match pinger {
        Some(pinger) if pinger.owed => {
            socket.start_send(ping())?;
            pinger.owed = false;
            pinger.unflushed = true;
            Ok(true)
        }
        Some(_) | None => Ok(false),
    }
}

/// An optional limit on silence: reset by every message, fires only while
/// the socket has nothing to read.
#[derive(Debug)]
pub(super) struct Idle {
    pub(super) after: Duration,
    deadline: Pin<Box<Sleep>>,
}

impl Idle {
    pub(super) fn new(after: Option<Duration>) -> Option<Self> {
        after.map(|after| Self {
            after,
            deadline: Box::pin(tokio::time::sleep(after)),
        })
    }

    pub(super) fn heard(this: &mut Option<Self>) {
        if let Some(idle) = this {
            let now = Instant::now();
            idle.deadline
                .as_mut()
                .reset(now.checked_add(idle.after).unwrap_or(now));
        }
    }

    /// Called when the socket is pending: an error once the silence is too
    /// long, `Pending` otherwise.
    pub(super) fn poll_expired<T>(
        this: &mut Option<Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<io::Result<T>>> {
        let Some(idle) = this else {
            return Poll::Pending;
        };
        if idle.deadline.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        }
        Poll::Ready(Some(Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "the connection went silent",
        ))))
    }
}
