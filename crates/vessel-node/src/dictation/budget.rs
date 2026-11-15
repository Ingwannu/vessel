//! Adaptive finish budget, without timers, stream mutation or audio buffers.
//!
//! Count the entire unacknowledged suffix, including held chunks. Open audio
//! contributes rounded seconds once; it does not include the held input again.

use std::time::Duration;
use vessel_proto::dictation::{ChunkSeq, WithinMs};

const BASE: Duration = Duration::from_secs(10);
const PER_PENDING: Duration = Duration::from_secs(15);
const PER_AUDIO_SECOND: Duration = Duration::from_millis(1_500);
const PER_SUFFIX_SEQUENCE: Duration = Duration::from_millis(250);

/// Keep a pending commit count distinct from bytes and sequence positions.
#[derive(Clone, Copy)]
pub(super) struct PendingCommits(usize);

impl PendingCommits {
    /// The text owner counts submitted slots without a final result.
    #[must_use]
    pub(super) const fn new(count: usize) -> Self {
        Self(count)
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum BudgetError {
    #[error("dictation finish counts exceeded this platform's range")]
    Count(#[source] std::num::TryFromIntError),
}

/// Freeze this result and its deadline on the first admitted finish.
pub(super) fn compute(
    pending: PendingCommits,
    open: Duration,
    through: Option<ChunkSeq>,
    last: ChunkSeq,
) -> Result<WithinMs, BudgetError> {
    let pending = u64::try_from(pending.0).map_err(BudgetError::Count)?;
    let suffix = match through {
        Some(through) => u64::from(last.get()).saturating_sub(u64::from(through.get())),
        None => u64::from(last.get()).saturating_add(1),
    };
    let seconds = open
        .as_secs()
        .saturating_add(u64::from(open.subsec_nanos() != 0));
    // Counts can grow without wrapping or restarting a timer: only the bounded
    // total leaves this arithmetic edge, even for future wider stream limits.
    let total = BASE
        .as_millis()
        .saturating_add(PER_PENDING.as_millis().saturating_mul(u128::from(pending)))
        .saturating_add(
            PER_AUDIO_SECOND
                .as_millis()
                .saturating_mul(u128::from(seconds)),
        )
        .saturating_add(
            PER_SUFFIX_SEQUENCE
                .as_millis()
                .saturating_mul(u128::from(suffix)),
        )
        .min(u128::from(WithinMs::MAX.get()));
    u32::try_from(total)
        .map(WithinMs::new)
        .map_err(BudgetError::Count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected(duration: Duration) -> WithinMs {
        WithinMs::new(u32::try_from(duration.as_millis()).unwrap())
    }

    #[test]
    fn an_empty_open_buffer_has_only_the_base_budget() {
        assert_eq!(
            compute(
                PendingCommits::new(0),
                Duration::ZERO,
                Some(ChunkSeq::new(0)),
                ChunkSeq::new(0)
            )
            .unwrap(),
            expected(BASE)
        );
    }

    #[test]
    fn pending_results_open_audio_and_the_whole_suffix_are_separate_terms() {
        let open = Duration::from_secs(2).saturating_sub(Duration::from_nanos(1));
        let result = compute(
            PendingCommits::new(2),
            open,
            Some(ChunkSeq::new(3)),
            ChunkSeq::new(5),
        )
        .unwrap();
        let wanted = BASE
            .saturating_add(PER_PENDING.saturating_mul(2))
            .saturating_add(PER_AUDIO_SECOND.saturating_mul(2))
            .saturating_add(PER_SUFFIX_SEQUENCE.saturating_mul(2));
        assert_eq!(result, expected(wanted));
    }

    #[test]
    fn no_contiguous_ack_counts_every_position_including_already_held_audio() {
        let result = compute(
            PendingCommits::new(0),
            Duration::ZERO,
            None,
            ChunkSeq::new(2),
        )
        .unwrap();
        assert_eq!(
            result,
            expected(BASE.saturating_add(PER_SUFFIX_SEQUENCE.saturating_mul(3)))
        );
    }

    #[test]
    fn fractional_seconds_round_up_but_whole_seconds_do_not_round_twice() {
        for open in [Duration::from_nanos(1), Duration::from_secs(1)] {
            let result = compute(
                PendingCommits::new(0),
                open,
                Some(ChunkSeq::new(0)),
                ChunkSeq::new(0),
            )
            .unwrap();
            assert_eq!(result, expected(BASE.saturating_add(PER_AUDIO_SECOND)));
        }
    }

    #[test]
    fn bounds_cap_without_overflow_and_a_suffix_never_wraps_below_zero() {
        assert_eq!(
            compute(
                PendingCommits::new(usize::MAX),
                Duration::MAX,
                None,
                ChunkSeq::new(u32::MAX)
            )
            .unwrap(),
            WithinMs::MAX
        );
        assert_eq!(
            compute(
                PendingCommits::new(0),
                Duration::ZERO,
                Some(ChunkSeq::new(2)),
                ChunkSeq::new(1)
            )
            .unwrap(),
            expected(BASE)
        );
    }
}
