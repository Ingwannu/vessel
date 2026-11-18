//! Places for listings of agents' histories (ADR 0109, replacing ADR
//! 0053's two places and nothing waiting).
//!
//! [Decision Log]
//! - Purpose: Paseo's import sheet reads every provider of a host at once
//!   (one read per provider, and per account here), and a ranked listing
//!   loads conversations for up to a minute. With two places and the rest
//!   answered `unavailable`, a host with three agents showed one as
//!   failed, so the native sheet queued its own reads (777).
//! - Alternatives: more places with nothing waiting (each place is an
//!   agent process, a few hundred MB; a sheet of many agents still fails);
//!   an unbounded queue (ADR 0053's worry: a caller waits without bound
//!   behind listings that each take up to [`vessel_agent::HISTORY_RANKED_LIMIT`]).
//! - Choice: [`AT_ONCE`] running and at most [`WAITING`] waiting, in
//!   order (tokio's semaphore is fair), each for at most [`WAIT`]; past
//!   either bound `unavailable`, as before. A ranked listing's previews
//!   are kept here too ([`Listings::previews`]), so they outlive one
//!   listing.
//! - Impact: a caller may wait up to [`WAIT`] before its listing starts;
//!   a client's deadline for a read covers both (the native sheet's).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::{Semaphore, SemaphorePermit};

/// Listings running at once, node-wide.
pub(in crate::service) const AT_ONCE: usize = 4;

/// Listings waiting for a place at once; more are `unavailable`.
pub(in crate::service) const WAITING: usize = 16;

/// How long a listing waits for a place: one ranked listing's bound.
pub(in crate::service) const WAIT: Duration = vessel_agent::HISTORY_RANKED_LIMIT;

/// The node's places for listings, and the previews they read.
#[derive(Debug)]
pub(in crate::service) struct Listings {
    places: Semaphore,
    waiting: AtomicUsize,
    /// Prompts read by ranked listings, per account and conversation.
    pub(in crate::service) previews: vessel_agent::Previews,
}

impl Default for Listings {
    fn default() -> Self {
        Self {
            places: Semaphore::new(AT_ONCE),
            waiting: AtomicUsize::new(0),
            previews: vessel_agent::Previews::default(),
        }
    }
}

impl Listings {
    /// A place to list in: at once when one is free, else after the
    /// listings waiting before this one; `None` when [`WAITING`] already
    /// wait or none freed within [`WAIT`].
    pub(in crate::service) async fn place(&self) -> Option<SemaphorePermit<'_>> {
        if let Ok(place) = self.places.try_acquire() {
            return Some(place);
        }
        let waiting = Waiting::join(&self.waiting)?;
        let place = tokio::time::timeout(WAIT, self.places.acquire()).await;
        drop(waiting);
        place.ok()?.ok()
    }
}

/// One listing counted among those waiting, until dropped.
struct Waiting<'a>(&'a AtomicUsize);

impl<'a> Waiting<'a> {
    fn join(count: &'a AtomicUsize) -> Option<Self> {
        count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |now| {
                (now < WAITING).then(|| now.saturating_add(1))
            })
            .ok()
            .map(|_before| Self(count))
    }
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        let _before = self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn four_list_at_once_sixteen_wait_in_order_and_more_are_refused() {
        let listings = Arc::new(Listings::default());
        let mut running = Vec::new();
        for _ in 0..AT_ONCE {
            running.push(listings.place().await.expect("a free place"));
        }
        let (done, mut started) = tokio::sync::mpsc::unbounded_channel();
        for n in 0..WAITING {
            let listings = Arc::clone(&listings);
            let done = done.clone();
            drop(tokio::spawn(async move {
                let place = listings.place().await;
                done.send((n, place.is_some())).unwrap();
                if place.is_some() {
                    // A listing that started holds its place.
                    std::future::pending::<()>().await;
                }
                drop(place);
            }));
            tokio::task::yield_now().await;
        }
        assert!(
            listings.place().await.is_none(),
            "a seventeenth waiting is refused at once"
        );
        assert!(started.try_recv().is_err(), "the sixteen still wait");

        drop(running.remove(0));
        assert_eq!(started.recv().await, Some((0, true)), "the first in line");

        // Nothing frees again: the rest give up after the wait.
        let at = tokio::time::Instant::now();
        let mut gave_up: usize = 0;
        while let Ok(Some((_, placed))) =
            tokio::time::timeout(WAIT.saturating_mul(2), started.recv()).await
        {
            assert!(!placed);
            gave_up = gave_up.saturating_add(1);
            if gave_up == WAITING.saturating_sub(1) {
                break;
            }
        }
        assert_eq!(gave_up, WAITING.saturating_sub(1));
        assert!(at.elapsed() <= WAIT, "none waited past {WAIT:?}");
        assert_eq!(listings.waiting.load(Ordering::Acquire), 0);
    }
}
