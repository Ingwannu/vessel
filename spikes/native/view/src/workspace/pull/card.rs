//! What a session's card says of its pull request: Paseo's sidebar meta
//! row (`components/sidebar/workspace-meta-row/index.tsx`): the state's
//! glyph and the number, the state's word only when it is not open (an
//! open one is the unremarkable case), and the checks' outcome as one
//! word (`check-summary.ts`: a failure wins over progress).
//!
//! Paseo's ring that fills by quarters needs every check's state; the
//! summary on the card carries the overall state only, so the glyph is
//! empty or full, as Paseo's own fallback (`summarizeChecksStatus`).

use vessel_proto::forge::{ChecksState, PullNumber, SessionPullRequest};

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
        })
    }
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

    use vessel_proto::forge::{ChecksState, ForgeAccess, PullState, SessionPullRequest};

    use super::{CardChecks, CardPull};
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
}
