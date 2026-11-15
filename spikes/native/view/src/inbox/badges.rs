//! Platform-independent status projection. Membership and root eligibility are
//! supplied by the workspace owner; retained session membership is not liveness.
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
/// A workspace's highest-priority eligible session/terminal state.
#[must_use]
pub fn workspace(states: impl IntoIterator<Item = Bucket>) -> Bucket {
    states.into_iter().min().unwrap_or(Bucket::Done)
}
/// Count workspaces, never requests or tabs. Callers must pass each canonical
/// workspace exactly once after applying its membership/root visibility rules.
#[must_use]
pub fn dock_count(workspaces: impl IntoIterator<Item = Bucket>) -> usize {
    workspaces
        .into_iter()
        .filter(|state| matches!(state, Bucket::NeedsInput | Bucket::Failed | Bucket::Review))
        .count()
}
/// Global icon priority differs from workspace priority: any running agent
/// wins, then attention. Native GPUI has no favicon/dock API at this checkpoint.
#[must_use]
pub fn icon(states: impl IntoIterator<Item = Bucket>) -> Icon {
    let mut result = Icon::Clear;
    for state in states {
        match state {
            Bucket::Working => return Icon::Running,
            Bucket::NeedsInput | Bucket::Failed | Bucket::Review => result = Icon::Attention,
            Bucket::Done => {}
        }
    }
    result
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
        ];
        assert_eq!(workspace(states), Bucket::NeedsInput);
        assert_eq!(icon(states), Icon::Running);
        assert_eq!(
            dock_count([workspace(states), Bucket::Done, Bucket::Working]),
            1
        );
        assert_eq!(workspace([]), Bucket::Done);
        assert_eq!(dock_count([]), 0);
        assert_eq!(icon([]), Icon::Clear);
        assert_eq!(icon([Bucket::Failed, Bucket::Review]), Icon::Attention);
    }
}
