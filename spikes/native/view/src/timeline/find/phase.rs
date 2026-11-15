//! Find's user-visible states are independent of where the transcript is kept.

/// Why a result cannot currently be used; retry starts from the current history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindFailure {
    /// The followed session lost its connection.
    Connection,
    /// Rewind replaced the history underlying the results.
    HistoryChanged,
    /// A located message failed to arrive or reveal before the deadline.
    Reveal,
}

/// The phases exposed by Paseo's find widget.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FindPhase {
    /// No nonempty query.
    #[default]
    Idle,
    /// Debouncing or scanning the transcript in slices.
    Searching,
    /// A selected search result has not arrived in the followed log yet.
    Loading,
    /// The count and selected occurrence refer to the current history.
    Ready,
    /// Results must not navigate until a retry or edit.
    Failed(FindFailure),
}

impl FindPhase {
    /// Only settled current results can navigate or paint marks.
    #[must_use]
    pub const fn ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}
