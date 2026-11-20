//! `terminal.list`'s revision (ADR 0116): one number for the node's list
//! of terminals that moves when a client showing them as tabs must look
//! again, so `terminal.list` with `changed_from` can wait for it.
//!
//! [Decision Log]
//! - Purpose: Paseo pushes a workspace's terminals (`terminals_changed`)
//!   after a create, an exit, a rename or a close; a second client sees a
//!   shell another one opened at once. Vessel's clients asked
//!   `terminal.list` every 2 s.
//! - Constraint: the node's subscriptions carry a log's records with their
//!   sequence numbers and replay; a list is not a record.
//! - Choice: a long poll, as `terminal.get` and `port.list` already are:
//!   a `watch` of one counter, moved by the registry (a terminal kept or
//!   forgotten) and by one small task per terminal that watches its info
//!   and moves the counter only when what a list shows changes (its
//!   status, title, service state or end, and its attention once ADR
//!   0089's feature is assigned), never for a resize or output.
//! - Attention (owed item 6 of the node review of TRACKER 815): a client
//!   showing a terminal's attention as a tab's badge must see it change.
//!   While the feature is unassigned every peer has it cut, so it is left
//!   out then: a hook looping Working/Idle would otherwise wake every
//!   waiter for a list that reads the same.
//! - Cost: the counter is the node's, not a place's, so a change beside
//!   another session wakes a waiter too, which reads the same list and
//!   asks again. An agent, which lists only the terminals it opened, is
//!   woken by others' changes as well: it learns when they happen, never
//!   what they are. One task per kept terminal (at most 96), ending with
//!   the terminal's handle or the node.

use tokio::sync::watch;
use vessel_proto::notifications::{ATTENTION_NOTIFICATIONS_PROTOCOL, TerminalAttention};
use vessel_proto::terminals::{ServiceState, TerminalInfo};
use vessel_proto::{SessionStatus, SessionTitle};
use vessel_session::terminals::TerminalHandle;

use super::Terminals;

/// What a list shows of a terminal that can change after it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    status: SessionStatus,
    title: Option<SessionTitle>,
    service: Option<ServiceState>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    /// `None` while ADR 0089's feature is unassigned (the Decision Log).
    attention: Option<TerminalAttention>,
}

impl Shown {
    fn of(info: &TerminalInfo) -> Self {
        Self {
            status: info.status.clone(),
            title: info.title.clone(),
            service: info.service.clone(),
            ended_at: info.ended_at,
            attention: info
                .attention
                .filter(|_| ATTENTION_NOTIFICATIONS_PROTOCOL.is_some()),
        }
    }
}

fn bump(revision: &mut u64) {
    *revision = revision.wrapping_add(1);
}

impl Terminals {
    /// The list's revision now.
    pub(crate) fn list_revision(&self) -> u64 {
        *self.listed.borrow()
    }

    /// A receiver of the list's revision, to wait for its next move.
    pub(crate) fn list_changes(&self) -> watch::Receiver<u64> {
        self.listed.subscribe()
    }

    /// The list changed: a terminal was kept or forgotten.
    pub(super) fn list_moved(&self) {
        self.listed.send_modify(bump);
    }

    /// Move the list's revision whenever what it shows of `handle`
    /// changes, until its handle is gone or the node stops.
    pub(super) fn follow_listed(&self, handle: &TerminalHandle) {
        let mut infos = handle.watch();
        let listed = std::sync::Arc::clone(&self.listed);
        let shutdown = self.runner.shutdown.clone();
        drop(self.runner.tasks.spawn(async move {
            let mut shown = Shown::of(&infos.borrow_and_update());
            loop {
                tokio::select! {
                    () = shutdown.cancelled() => return,
                    changed = infos.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
                let now = Shown::of(&infos.borrow_and_update());
                if now != shown {
                    shown = now;
                    listed.send_modify(bump);
                }
            }
        }));
    }
}
