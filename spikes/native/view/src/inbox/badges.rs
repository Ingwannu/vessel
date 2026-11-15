//! Platform-independent status projection. Membership and root eligibility are
//! supplied by the workspace owner; unknown facts are never silently zero/Done.
use super::buckets::Bucket;

/// Aggregate icon state used by browser/desktop host adapters when supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Any active work takes precedence for the global application icon.
    Running,
    /// A request, failure or unseen result needs the person.
    Attention,
    /// No active work or attention.
    Clear,
}
/// A count of eligible canonical workspaces, independent of requests or tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceCount(usize);
impl WorkspaceCount {
    /// Value accepted by a future desktop host badge adapter.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
/// A workspace's highest-priority eligible state. An empty known workspace is
/// Done; any unknown ownership/attention input remains explicitly unknown.
#[must_use]
pub fn workspace(states: impl IntoIterator<Item = Option<Bucket>>) -> Option<Bucket> {
    let mut best = Bucket::Done;
    for state in states {
        best = best.min(state?);
    }
    Some(best)
}
/// Count workspaces, never requests or tabs. Each workspace appears once after
/// canonical membership/root filtering. Unknown workspaces do not yield zero.
#[must_use]
pub fn dock_count(workspaces: impl IntoIterator<Item = Option<Bucket>>) -> Option<WorkspaceCount> {
    let mut count = 0usize;
    for state in workspaces {
        if matches!(state?, Bucket::NeedsInput | Bucket::Failed | Bucket::Review) {
            count = count.checked_add(1)?;
        }
    }
    Some(WorkspaceCount(count))
}
/// Global icon priority differs from workspace priority: any running agent
/// wins, then attention. Unknown facts remain unknown even beside active work.
#[must_use]
pub fn icon(states: impl IntoIterator<Item = Option<Bucket>>) -> Option<Icon> {
    let mut result = Icon::Clear;
    for state in states {
        match state? {
            Bucket::Working => result = Icon::Running,
            Bucket::NeedsInput | Bucket::Failed | Bucket::Review => {
                if result != Icon::Running {
                    result = Icon::Attention;
                }
            }
            Bucket::Done => {}
        }
    }
    Some(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_and_global_icon_use_their_distinct_reference_priorities() {
        let states = [
            Bucket::Working,
            Bucket::NeedsInput,
            Bucket::Failed,
            Bucket::Review,
        ]
        .map(Some);
        assert_eq!(workspace(states), Some(Bucket::NeedsInput));
        assert_eq!(icon(states), Some(Icon::Running));
        assert_eq!(
            dock_count([workspace(states), Some(Bucket::Done), Some(Bucket::Working)])
                .unwrap()
                .get(),
            1
        );
        assert_eq!(workspace([]), Some(Bucket::Done));
        assert_eq!(dock_count([]).unwrap().get(), 0);
        assert_eq!(icon([]), Some(Icon::Clear));
        assert_eq!(
            icon([Some(Bucket::Failed), Some(Bucket::Review)]),
            Some(Icon::Attention)
        );
    }
    #[test]
    fn unknown_membership_or_attention_is_neither_done_nor_zero() {
        for states in [
            [Some(Bucket::Working), None],
            [None, Some(Bucket::NeedsInput)],
        ] {
            assert_eq!(workspace(states), None);
            assert_eq!(icon(states), None);
            assert_eq!(dock_count(states), None);
        }
    }
}
