//! A session's kept row metadata while the runtime has it (protocol 10, ADR 0026):
//! the node's copy of what the store keeps, and the last record that asked
//! for attention, so the session's info carries the same marks the store
//! would give it, at once. Its mutation clock is the latest acknowledged
//! durable value, deliberately not an atomic snapshot of those actor fields.
//!
//! Owns: the [`KeptMetadata`] in [`Shared`], updated by the publisher (attention)
//! and by the node ([`SessionHandle::set_marks`]), and adding them to an
//! info ([`KeptMetadata::apply`]). Must not: decide a mark (the node applies it
//! in the store first and hands the result here), or derive `unread` any
//! other way than `KeptMarks::marks`, which the store uses too.
//!
//! [Decision Log]
//! - Purpose: `session.list`, `session.get`, a subscription's info and the
//!   board must agree about a session the runtime has, and agree with the
//!   store once the session is handed over to it.
//! - Constraint: the store is the keeper (marks outlive the node), but its
//!   copy of a live session's log lags by a commit, and the session's info
//!   watch closes when the session ends, while a mark (`archive` above
//!   all) arrives after the end.
//! - Alternatives: read the marks from the store for every info of a live
//!   session (a store read per `session.get`, and an unread that lags the
//!   records a client already has); put them in the info watch (its sender
//!   is gone once the session ended, so an archive could not reach it).
//! - Choice: the store's marks are copied here after each mark, and the
//!   attention point is read off the records as the publisher commits them
//!   (the same rule the store's projection uses,
//!   `SessionEvent::asks_for_attention`). [`SessionHandle::info`] adds both
//!   to the watch's value, so every info of the session carries them, and
//!   an archived one is placed in the archive.
//! - Cost: one short lock per record that asks for attention and per info
//!   read. The info watch itself carries no marks: it wakes on what the
//!   session's records change.

use std::sync::{MutexGuard, PoisonError};

use vessel_proto::{EventSeq, KeptMarks, SessionBoard, SessionInfo, SessionUpdatedAt};

use crate::handle::{SessionHandle, Shared};

/// The bounded runtime overlay of kept marks, ancestry and durable time.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KeptMetadata {
    /// The store's, as of the last mark the node applied.
    kept: KeptMarks,
    /// The separately durable current-parent boundary.
    detached: Option<vessel_proto::subagents::DetachReceipt>,
    /// The last record asking for attention, as published.
    attention: Option<EventSeq>,
    /// The latest acknowledged durable clock, not a revision of the other fields.
    updated_at: Option<SessionUpdatedAt>,
}

impl KeptMetadata {
    /// A session started again (protocol 10): `attention` is the last
    /// record of its stored log that asked for attention; the store's
    /// marks follow once its row is reopened ([`SessionHandle::set_marks`]).
    pub(crate) fn after(
        attention: Option<EventSeq>,
        detached: Option<vessel_proto::subagents::DetachReceipt>,
        updated_at: Option<SessionUpdatedAt>,
    ) -> Self {
        Self {
            kept: KeptMarks::default(),
            detached,
            attention,
            updated_at,
        }
    }

    /// The record `seq`, about to be published, asks for attention
    /// (`SessionEvent::asks_for_attention`).
    pub(crate) const fn asked(&mut self, seq: EventSeq) {
        self.attention = Some(seq);
    }

    /// Add the marks to `info`, and place an archived session that has
    /// ended in the archive: the reducer's first rule (`is_terminated` is
    /// archived, ADR 0026), which the publisher cannot apply itself because
    /// a session is archived only once its records have ended.
    pub(crate) fn apply(&self, info: &mut SessionInfo) {
        info.marks = self.kept.marks(self.attention);
        info.detach_receipt = self.detached;
        info.updated_at = self.updated_at;
        if self.kept.is_archived() && !info.status.is_live() {
            // An ended session has its end (the final status sets it); its
            // creation is a floor, as the store's.
            let ended_at = info.ended_at.unwrap_or(info.created_at);
            info.board = Some(SessionBoard::archived(ended_at));
        }
    }

    // A later append ACK can arrive before an earlier mark's live publication.
    // Keep the clock monotonic, but never use it to discard that mark's state.
    fn observe_time(&mut self, updated_at: Option<SessionUpdatedAt>) {
        if let Some(updated) = updated_at {
            self.updated_at = Some(self.updated_at.map_or(updated, |kept| kept.max(updated)));
        }
    }

    fn keep_marks(&mut self, kept: KeptMarks, updated_at: Option<SessionUpdatedAt>) {
        self.kept = kept;
        self.observe_time(updated_at);
    }

    fn keep_detach(
        &mut self,
        receipt: Option<vessel_proto::subagents::DetachReceipt>,
        updated_at: Option<SessionUpdatedAt>,
    ) {
        self.detached = receipt;
        self.observe_time(updated_at);
    }
}

impl Shared {
    /// The marks, recovering from a poisoned lock: each update is one
    /// assignment, so the value is whole.
    pub(crate) fn kept_metadata(&self) -> MutexGuard<'_, KeptMetadata> {
        self.kept_metadata
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl SessionHandle {
    /// Take the marks the store now keeps for this session (protocol 10's
    /// `session.mark`, applied there first). The node calls this in the
    /// order its marks were stored, so the last one stored is the one
    /// kept. Works on an ended session too, until the runtime forgets it.
    pub fn set_marks(&self, kept: KeptMarks) {
        self.0.kept_metadata().keep_marks(kept, None);
    }

    /// Publish only the receipt the store has committed, serialized by the node
    /// with marks and resume. This never changes creation provenance or the actor.
    pub fn set_detach_receipt(&self, receipt: Option<vessel_proto::subagents::DetachReceipt>) {
        self.0.kept_metadata().keep_detach(receipt, None);
    }

    /// The end of the session's log as published: the `seq` the next
    /// record gets. A record a follower has received is always before it
    /// (the ring is appended before anyone is sent the record), so a
    /// `read` through it is never ahead of what that follower saw.
    #[must_use]
    pub fn log_end(&self) -> EventSeq {
        self.0.ring().next()
    }
}

impl SessionHandle {
    /// Publish a local writer's acknowledged time, even after the actor ended.
    /// Delayed/Unknown replies never regress a known clock. This changes no
    /// process, label revision or permission; only the node calls it after commit.
    pub fn acknowledge_time(&self, updated_at: Option<SessionUpdatedAt>) {
        self.0.kept_metadata().observe_time(updated_at);
    }

    /// Apply kept marks in the node's existing mark order and retain their
    /// acknowledged time under the same overlay lock. A newer clock alone
    /// cannot justify discarding a still-unpublished earlier mark.
    pub fn set_committed_marks(&self, kept: KeptMarks, updated_at: Option<SessionUpdatedAt>) {
        self.0.kept_metadata().keep_marks(kept, updated_at);
    }

    /// Retain the canonical detach receipt and its acknowledged row time,
    /// without deriving ancestry or authority from that clock.
    pub fn set_committed_detach(
        &self,
        receipt: Option<vessel_proto::subagents::DetachReceipt>,
        updated_at: Option<SessionUpdatedAt>,
    ) {
        self.0.kept_metadata().keep_detach(receipt, updated_at);
    }
}

#[cfg(test)]
mod tests;
