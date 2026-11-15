//! Shared Plan disclosure/outcome values, without any permission action.

/// The plan card's title and initial disclosure state follow its decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanOutcome {
    /// No permission decision has been observed.
    Proposed,
    /// Its permission is still actionable in the dock.
    Pending,
    /// The person selected an offered allow action.
    Approved,
    /// The person selected an offered reject action.
    Rejected,
    /// The request was withdrawn by the provider.
    Canceled,
}

impl PlanOutcome {
    /// Rejected and canceled plans start folded, as Paseo's `PlanCard` does.
    #[must_use]
    pub const fn expanded(self) -> bool {
        match self {
            Self::Proposed | Self::Pending | Self::Approved => true,
            Self::Rejected | Self::Canceled => false,
        }
    }
}

/// The disclosure is explicit because the dock and stream own separate state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanExpansion {
    /// The full Markdown is visible.
    Expanded,
    /// Only the decision title is visible.
    Collapsed,
}

impl PlanExpansion {
    /// A fresh outcome starts with Paseo's default disclosure.
    #[must_use]
    pub const fn for_outcome(outcome: PlanOutcome) -> Self {
        if outcome.expanded() {
            Self::Expanded
        } else {
            Self::Collapsed
        }
    }

    /// Tapping changes only the local disclosure, never the approval.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Expanded => Self::Collapsed,
            Self::Collapsed => Self::Expanded,
        }
    }
}
