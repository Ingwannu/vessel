//! Bounded speaker ownership and drain receipts, before native output wiring.
//!
//! [Decision Log]
//! - Purpose: queued PCM and a callback's final copy are not completed playback.
//! - Constraint: Stop/barge-in must cancel full queues without waiting on them.
//! - Choice: at most two segment leases, each with an independent atomic fence;
//!   a terminal Played receipt requires the predicted device drain deadline.
//! - Why: the same leases fence prepared blocks, callbacks and late drain events.
//! - Cost: a missing/invalid device clock fails the segment rather than claiming
//!   completion. Native callback timing and physical output remain separate proof.

pub(in crate::audio) mod control;
pub(super) mod cue;
pub(super) mod output;
pub(super) mod packet;
pub(super) mod pump;
pub(super) mod receipts;

use receipts::Refusal;
