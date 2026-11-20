//! What a session's card says of its pull request: Paseo's sidebar meta
//! row (`components/sidebar/workspace-meta-row/index.tsx`): the state's
//! glyph and the number, the state's word only when it is not open (an
//! open one is the unremarkable case), and the checks' outcome as one
//! word (`check-summary.ts`: a failure wins over progress).
//!
//! The checks' glyph is Paseo's `CheckIndicator` (TRACKER 426): one ring
//! whose inside says the outcome, a pie filling by quarters while the run
//! goes on (`check-summary.ts` `toFilledQuarters`).
//!
//! [Decision Log]
//! - Purpose: a running row shows how far its checks have come, as
//!   Paseo's sidebar does, not a dot that says only "running".
//! - Constraint: Paseo counts its pie from every check's status; the node
//!   sends the summary's state and, since protocol 39, its tally (passed,
//!   failed, pending), not the list.
//! - Choice: the pie's done share is passed and failed of the tally's
//!   total, rounded down to a quarter (nine of ten is three quarters, only
//!   the tenth closes it); the outcome stays the summary's state, so a
//!   failure still wins over progress. Without a tally (an older node, or
//!   none counted) the ring stays empty, Paseo's own fallback
//!   (`summarizeChecksStatus`: running reads 0 of 1).
//! - Differences: the tally counts skipped and neutral as passed, as the
//!   node does; Paseo counts them as done too, so the pie agrees.

use vessel_proto::forge::{CheckTally, ChecksState, PullNumber, SessionPullRequest};

use super::facts::PullPhase;

/// The checks' outcome as the card says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardChecks {
    /// Every check passed.
    Passed,
    /// One failed (or was cancelled).
    Failed,
    /// Some still run.
    Running,
}

/// The card's pull request item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardPull {
    /// Its number.
    pub number: PullNumber,
    /// Where it is in its life.
    pub phase: PullPhase,
    /// Whether its state's word follows the number (not when open).
    pub says_state: bool,
    /// Its checks, when it has any the node read.
    pub checks: Option<CardChecks>,
    /// How many quarters of the running pie are filled (0 to 4), from the
    /// node's tally; 0 without one.
    pub quarters: u8,
}

impl CardPull {
    /// The item for `pull`, when one has been read.
    #[must_use]
    pub fn of(pull: &SessionPullRequest) -> Option<Self> {
        let summary = pull.summary.as_ref()?;
        let phase = PullPhase::of(summary);
        Some(Self {
            number: summary.number,
            phase,
            says_state: phase != PullPhase::Open,
            checks: match summary.checks {
                ChecksState::Failing | ChecksState::Cancelled => Some(CardChecks::Failed),
                ChecksState::Pending => Some(CardChecks::Running),
                ChecksState::Passing => Some(CardChecks::Passed),
                ChecksState::None | ChecksState::Unknown => None,
            },
            quarters: quarters(summary.tally),
        })
    }
}

/// The share of `tally` done (passed or failed), in whole quarters rounded
/// down (Paseo's `toFilledQuarters`, four pieces).
fn quarters(tally: Option<CheckTally>) -> u8 {
    let Some(tally) = tally else {
        return 0;
    };
    let done = u64::from(tally.passed).saturating_add(u64::from(tally.failed));
    let total = done.saturating_add(u64::from(tally.pending));
    let filled = done.saturating_mul(4).checked_div(total).unwrap_or(0);
    u8::try_from(filled.min(4)).unwrap_or(4)
}

impl CardChecks {
    /// Paseo's one word (`checksSummary.passedLabel` and its kin).
    #[must_use]
    pub fn words(self, catalog: &crate::localization::Translator) -> &str {
        let words = catalog.pulls();
        match self {
            Self::Passed => words.card_passed(),
            Self::Failed => words.card_failed(),
            Self::Running => words.card_running(),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Paseo's rules: open says only its number; merged and closed say
    //! their state; a failure wins; no checks, no item.

    use vessel_proto::forge::{
        CheckTally, ChecksState, ForgeAccess, PullState, SessionPullRequest,
    };

    use super::{CardChecks, CardPull, quarters};
    use crate::workspace::pull::PullPhase;
    use crate::workspace::pull::fixtures::summary;

    fn pull(state: PullState, checks: ChecksState) -> SessionPullRequest {
        let mut read = summary(state, false);
        read.checks = checks;
        SessionPullRequest {
            access: ForgeAccess::Ready { login: None },
            summary: Some(read),
            feedback: false,
        }
    }

    #[test]
    fn an_open_one_says_only_its_number_and_a_merged_one_its_state() {
        let open = CardPull::of(&pull(PullState::Open, ChecksState::Passing)).unwrap();
        assert!(!open.says_state);
        assert_eq!(open.checks, Some(CardChecks::Passed));
        let merged = CardPull::of(&pull(PullState::Merged, ChecksState::None)).unwrap();
        assert!(merged.says_state);
        assert_eq!(merged.phase, PullPhase::Merged);
        assert_eq!(merged.checks, None);
    }

    #[test]
    fn a_cancelled_run_reads_as_failed_and_nothing_read_gives_no_item() {
        let cancelled = CardPull::of(&pull(PullState::Open, ChecksState::Cancelled)).unwrap();
        assert_eq!(cancelled.checks, Some(CardChecks::Failed));
        let unread = SessionPullRequest {
            access: ForgeAccess::Ready { login: None },
            summary: None,
            feedback: false,
        };
        assert_eq!(CardPull::of(&unread), None);
    }

    #[test]
    fn the_pie_fills_by_quarters_rounded_down() {
        let tally = |passed, failed, pending| {
            Some(CheckTally {
                passed,
                failed,
                pending,
            })
        };
        assert_eq!(quarters(tally(0, 0, 4)), 0);
        assert_eq!(quarters(tally(1, 0, 3)), 1);
        assert_eq!(quarters(tally(2, 0, 3)), 1);
        assert_eq!(quarters(tally(1, 1, 2)), 2);
        assert_eq!(quarters(tally(9, 0, 1)), 3);
        assert_eq!(quarters(tally(10, 0, 0)), 4);
        assert_eq!(quarters(None), 0);
    }

    #[test]
    fn a_running_pull_carries_its_tally_s_quarters() {
        let mut read = pull(PullState::Open, ChecksState::Pending);
        if let Some(summary) = read.summary.as_mut() {
            summary.tally = Some(CheckTally {
                passed: 3,
                failed: 0,
                pending: 1,
            });
        }
        let card = CardPull::of(&read).unwrap();
        assert_eq!(card.checks, Some(CardChecks::Running));
        assert_eq!(card.quarters, 3);
        let older = CardPull::of(&pull(PullState::Open, ChecksState::Pending)).unwrap();
        assert_eq!(older.quarters, 0);
    }
}
