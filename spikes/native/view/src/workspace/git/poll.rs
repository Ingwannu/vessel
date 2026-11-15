//! Refresh Git's facts while the code panel is open, so work done by an
//! agent can change an idle menu into Commit without reopening a session.

use super::{Reading, Writing};
use std::time::Duration;
use vessel_proto::workspace::GitStatus;

/// The page's visible-panel refresh cadence.
pub const REFRESH_EVERY: Duration = Duration::from_secs(5);

/// Whether the session's code panel is currently shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelVisibility {
    /// No request is useful for a hidden or different session.
    Hidden,
    /// Keep the current repository's facts fresh.
    Shown,
}

/// Do not overlap reads/writes. A failed read stays visibly failed until
/// the person retries, rather than clearing its guidance every five
/// seconds; a successful read resumes polling.
#[must_use]
pub fn should_refresh(
    status: &Reading<GitStatus>,
    writing: &Writing,
    visibility: PanelVisibility,
) -> bool {
    if visibility == PanelVisibility::Hidden || writing.busy() {
        return false;
    }
    match status {
        Reading::Empty | Reading::Ready(_) => true,
        Reading::Loading { .. } | Reading::Failed(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{RequestKeys, fixtures};
    use super::*;
    use crate::workspace::problem::Problem;

    #[test]
    fn only_visible_nonoverlapping_reads_refresh_and_failures_keep_their_guidance() {
        let mut read = Reading::Ready(fixtures::status(vec![]));
        let mut writing = Writing::default();
        assert!(should_refresh(&read, &writing, PanelVisibility::Shown));
        assert!(should_refresh(
            &Reading::Empty,
            &writing,
            PanelVisibility::Shown
        ));
        assert!(!should_refresh(&read, &writing, PanelVisibility::Hidden));
        assert!(!should_refresh(
            &Reading::Failed(Problem::Older),
            &writing,
            PanelVisibility::Shown
        ));
        let mut keys = RequestKeys::default();
        read.start(keys.issue().unwrap());
        assert!(!should_refresh(&read, &writing, PanelVisibility::Shown));
        assert!(writing.begin(keys.issue().unwrap()));
        assert!(!should_refresh(
            &Reading::Ready(fixtures::status(vec![])),
            &writing,
            PanelVisibility::Shown
        ));
        assert_eq!(REFRESH_EVERY, Duration::from_secs(5));
    }
}
