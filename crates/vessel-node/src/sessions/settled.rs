//! A barrier across the node's live sessions: what each had produced when
//! a read across sessions arrived is committed before the read, within a
//! bound (usage by machine or agent; gate 78b's `usage.spec.ts:127`).
//!
//! [Decision Log]
//! - Purpose: a person sees a turn end in the live stream, opens Usage, and
//!   the machine's totals must include that turn. Session scope already
//!   waits for its one session's commit (`session.read`'s barrier); machine
//!   and agent scope read the store at once, so under load the persist task
//!   could still be committing the turn and the totals missed it (the page
//!   asks once per selection, so it stayed missing).
//! - Choice (coordinator, 2026-11-18): every live session's commit of what
//!   it had produced when the call arrived, waited for concurrently, at
//!   most [`SETTLE`]. Past that the read goes on with what is stored, and
//!   the sessions still behind are named once in the log: a stalled store
//!   must not hang a usage page.
//! - Alternatives: the page re-asking until the totals move (it cannot
//!   know what to wait for, and other clients would still read a stale
//!   ledger); waiting without a bound (one stuck session would hang every
//!   machine read); waiting one session after another (the bound would
//!   grow with the number of live sessions).
//! - Cost: a machine or agent read waits for the slowest live session's
//!   pending commit, usually well under a millisecond, at most `SETTLE`.
//!   A session whose store failed (`Broken`) is not waited for: its later
//!   records will never be stored.

use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinSet;
use vessel_proto::{EventSeq, SessionId};

use super::Sessions;
use crate::persist::{self, Durability};

/// How long a read across sessions waits for the live ones to commit.
pub(crate) const SETTLE: Duration = Duration::from_secs(2);

impl Sessions {
    /// Wait, at most [`SETTLE`], until every live session has committed
    /// what it had produced when this was called; the sessions still
    /// behind then.
    pub(crate) async fn settled(&self) -> Vec<SessionId> {
        let waits: Vec<(SessionId, EventSeq, watch::Receiver<Durability>)> = self
            .durability
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter_map(|(id, durability)| {
                let produced = self.runtime.get(*id)?.info().next_seq;
                Some((*id, produced, durability.clone()))
            })
            .collect();
        settle(waits, SETTLE).await
    }
}

/// Wait for each session's durability to reach what it had produced, all
/// at once and at most `limit`; the ones that did not.
pub(crate) async fn settle(
    waits: Vec<(SessionId, EventSeq, watch::Receiver<Durability>)>,
    limit: Duration,
) -> Vec<SessionId> {
    let mut late: Vec<SessionId> = waits.iter().map(|(id, _, _)| *id).collect();
    let mut waiting = JoinSet::new();
    for (id, produced, mut durability) in waits {
        drop(waiting.spawn(async move {
            // A broken store ends the wait too: there is nothing to wait for.
            let _committed = persist::wait_until(id, &mut durability, produced).await;
            id
        }));
    }
    let deadline = tokio::time::Instant::now()
        .checked_add(limit)
        .unwrap_or_else(tokio::time::Instant::now);
    while !late.is_empty() {
        match tokio::time::timeout_at(deadline, waiting.join_next()).await {
            Ok(Some(Ok(id))) => late.retain(|behind| *behind != id),
            // A wait that panicked stays late; the others go on.
            Ok(Some(Err(_panicked))) => {}
            Ok(None) | Err(_) => break,
        }
    }
    late
}

#[cfg(test)]
mod tests;
