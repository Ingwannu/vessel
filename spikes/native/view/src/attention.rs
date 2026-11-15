//! Platform-independent status projection. Membership and root eligibility are
//! supplied by the workspace owner; unknown facts are never silently zero/Done.

/// Highest-priority current state of a session or workspace rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bucket {
    /// A current permission/question remains pending, even when seen.
    NeedsInput,
    /// A current failed turn or process, independently of read state.
    Failed,
    /// Active work.
    Working,
    /// Finished attention remains unseen.
    Review,
    /// Nothing pending or unseen.
    Done,
}
impl Bucket {
    /// Aggregate urgency order. Sidebar display/shortcut groups separately
    /// place review before working, following their own reference order.
    pub const ALL: [Self; 5] = [
        Self::NeedsInput,
        Self::Failed,
        Self::Working,
        Self::Review,
        Self::Done,
    ];
}

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
/// The workspace owner's latest eligible non-archived root selection. Historical
/// roots are not combined: only this one agent participates beside terminals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatestRoot {
    /// Membership and roles are known and there is no eligible root.
    Absent,
    /// Status of the one latest eligible root selected by the workspace owner.
    Known(Bucket),
    /// Membership, role or current attention could not be established.
    Unknown,
}
/// A terminal's contribution, distinct from the row's own activity indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    /// A known plain shell has no semantic producer/activity; no row dot and
    /// no contribution to workspace priority. This never means terminal Done.
    Untracked,
    /// Explicit supported activity or failure contributes this bucket.
    Known(Bucket),
    /// Missing, unavailable, future or incomplete facts cannot become neutral.
    Unknown,
}
/// Combine the latest eligible root with terminal statuses, never all historical
/// roots. The workspace owner preserves attentionTimestamp/updatedAt ordering.
#[must_use]
pub fn workspace(
    latest: LatestRoot,
    terminals: impl IntoIterator<Item = TerminalState>,
) -> Option<Bucket> {
    let mut best = match latest {
        // Paseo session.ts:5714/5811 seeds the descriptor as Done;
        // workspace-directory.ts:424-429 skips uninstrumented terminals and
        // :478-493 explicitly retains the empty-workspace Done state.
        LatestRoot::Absent => Bucket::Done,
        LatestRoot::Known(state) => state,
        LatestRoot::Unknown => return None,
    };
    for state in terminals {
        match state {
            TerminalState::Untracked => {}
            TerminalState::Known(state) => best = best.min(state),
            TerminalState::Unknown => return None,
        }
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
        assert_eq!(
            workspace(
                LatestRoot::Known(Bucket::NeedsInput),
                states.map(|state| TerminalState::Known(state.unwrap()))
            ),
            Some(Bucket::NeedsInput)
        );
        assert_eq!(icon(states), Some(Icon::Running));
        assert_eq!(
            dock_count([
                workspace(
                    LatestRoot::Known(Bucket::NeedsInput),
                    states.map(|state| TerminalState::Known(state.unwrap()))
                ),
                Some(Bucket::Done),
                Some(Bucket::Working)
            ])
            .unwrap()
            .get(),
            1
        );
        assert_eq!(workspace(LatestRoot::Absent, []), Some(Bucket::Done));
        assert_eq!(dock_count([]).unwrap().get(), 0);
        assert_eq!(icon([]), Some(Icon::Clear));
        assert_eq!(
            icon([Some(Bucket::Failed), Some(Bucket::Review)]),
            Some(Icon::Attention)
        );
    }
    #[test]
    fn unknown_membership_or_attention_is_neither_done_nor_zero() {
        assert_eq!(
            workspace(
                LatestRoot::Unknown,
                [TerminalState::Known(Bucket::NeedsInput)]
            ),
            None
        );
        for states in [
            [Some(Bucket::Working), None],
            [None, Some(Bucket::NeedsInput)],
        ] {
            assert_eq!(
                workspace(
                    LatestRoot::Absent,
                    states.map(|state| state.map_or(TerminalState::Unknown, TerminalState::Known))
                ),
                None
            );
            assert_eq!(icon(states), None);
            assert_eq!(dock_count(states), None);
        }
    }
}

#[cfg(test)]
mod terminal_projection_tests {
    use super::*;
    #[test]
    fn untracked_plain_shell_is_neutral_but_unavailable_attention_is_not() {
        for state in [Bucket::Working, Bucket::NeedsInput] {
            assert_eq!(
                workspace(LatestRoot::Known(state), [TerminalState::Untracked]),
                Some(state)
            );
        }
        assert_eq!(
            workspace(LatestRoot::Known(Bucket::Done), [TerminalState::Unknown]),
            None
        );
        // Source-backed empty-workspace rule, not a claim that the shell ended:
        // Paseo workspace-directory.test.ts:525-530 explicitly expects Done
        // for no agent plus an uninstrumented terminal with null activity.
        assert_eq!(
            workspace(LatestRoot::Absent, [TerminalState::Untracked]),
            Some(Bucket::Done)
        );
        assert_eq!(
            workspace(LatestRoot::Unknown, [TerminalState::Untracked]),
            None
        );
        assert_eq!(
            workspace(LatestRoot::Absent, [TerminalState::Unknown]),
            None
        );
    }
}
