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
    /// Explicit attention or a pending permission needs the person.
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
/// Latest eligible root for the app activity DISPLAY index only. This value
/// cannot represent the complete workspace aggregate contributor set.
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
/// One current agent's contribution after canonical workspace-relative role and
/// ownership resolution. The workspace owner supplies ALL active contributors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentContribution {
    /// Every eligible workspace-relative root contributes its current bucket.
    Root(Bucket),
    /// A same-workspace descendant contributes only while actually working.
    DescendantWorking,
    /// A known non-working same-workspace descendant contributes nothing.
    Neutral,
    /// Current membership, role or attention coverage could not be established.
    Unknown,
}
/// Combine ALL eligible roots and working descendants with terminal statuses.
/// Latest-root display selection and status timestamps never select the winner.
#[must_use]
pub fn workspace(
    agents: impl IntoIterator<Item = AgentContribution>,
    terminals: impl IntoIterator<Item = TerminalState>,
) -> Option<Bucket> {
    // The reference seeds Done and skips known non-contributing activity;
    // incomplete coverage remains Unknown rather than silently becoming empty.
    let mut best = Bucket::Done;
    for agent in agents {
        let state = match agent {
            AgentContribution::Root(state) => state,
            AgentContribution::DescendantWorking => Bucket::Working,
            AgentContribution::Neutral => continue,
            AgentContribution::Unknown => return None,
        };
        best = best.min(state);
    }
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
/// Raw agent activity retained independently of its attention bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconActivity {
    /// The aggregated agent's raw status is running.
    Running,
    /// The aggregated agent's raw status is not running.
    NotRunning,
}
/// Independent requires-attention OR pending-permission fact for the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconAttention {
    /// Either source fact requires attention.
    Required,
    /// Both source facts are known clear, even if the agent failed.
    Clear,
}
/// Icon inputs preserve facts a sidebar/workspace bucket has already discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconFacts {
    /// Raw running status; pending input must not overwrite this fact.
    pub activity: IconActivity,
    /// Attention or pending permission independently of activity/failure.
    pub attention: IconAttention,
}
/// Any raw running agent wins, then explicit attention/pending input. Unknown
/// facts remain unknown. A Failed bucket alone is not an icon attention fact.
#[must_use]
pub fn icon(states: impl IntoIterator<Item = Option<IconFacts>>) -> Option<Icon> {
    let mut result = Icon::Clear;
    for state in states {
        let state = state?;
        match (state.activity, state.attention) {
            (IconActivity::Running, IconAttention::Required | IconAttention::Clear) => {
                result = Icon::Running;
            }
            (IconActivity::NotRunning, IconAttention::Required) if result != Icon::Running => {
                result = Icon::Attention;
            }
            (IconActivity::NotRunning, IconAttention::Required | IconAttention::Clear) => {}
        }
    }
    Some(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_keeps_all_roots_and_working_descendants_without_timestamp_selection() {
        // Older input root and newest done root: display may choose Done,
        // but the complete workspace aggregate must retain NeedsInput.
        assert_eq!(
            workspace(
                [
                    AgentContribution::Root(Bucket::NeedsInput),
                    AgentContribution::Root(Bucket::Done),
                ],
                []
            ),
            Some(Bucket::NeedsInput)
        );
        assert_eq!(
            workspace(
                [
                    AgentContribution::Root(Bucket::Done),
                    AgentContribution::DescendantWorking,
                    AgentContribution::Neutral,
                ],
                []
            ),
            Some(Bucket::Working)
        );
        // Only the owner-resolved contributors enter this iterator. A foreign
        // workspace's input root cannot be supplied to this workspace's fold.
        let owned = [AgentContribution::Root(Bucket::Done)];
        assert_eq!(workspace(owned, []), Some(Bucket::Done));
        assert_eq!(
            workspace(
                [
                    AgentContribution::Root(Bucket::NeedsInput),
                    AgentContribution::Unknown,
                ],
                []
            ),
            None
        );
    }
    #[test]
    fn untracked_is_neutral_only_for_workspace_and_unknown_is_not_empty() {
        for state in [Bucket::Working, Bucket::NeedsInput] {
            assert_eq!(
                workspace([AgentContribution::Root(state)], [TerminalState::Untracked]),
                Some(state)
            );
        }
        assert_eq!(
            workspace([], [TerminalState::Untracked]),
            Some(Bucket::Done)
        );
        assert_eq!(
            workspace([AgentContribution::Unknown], [TerminalState::Untracked]),
            None
        );
        assert_eq!(
            workspace(
                [AgentContribution::Root(Bucket::Done)],
                [TerminalState::Unknown]
            ),
            None
        );
        assert_eq!(workspace([], []), Some(Bucket::Done));
        assert_eq!(
            dock_count([
                Some(Bucket::NeedsInput),
                Some(Bucket::Done),
                Some(Bucket::Working)
            ])
            .unwrap()
            .get(),
            1
        );
        assert_eq!(dock_count([]).unwrap().get(), 0);
        assert_eq!(dock_count([Some(Bucket::NeedsInput), None]), None);
    }
    #[test]
    fn icon_preserves_single_agent_raw_running_and_independent_attention() {
        let running_pending = IconFacts {
            activity: IconActivity::Running,
            attention: IconAttention::Required,
        };
        let failed_without_attention = IconFacts {
            activity: IconActivity::NotRunning,
            attention: IconAttention::Clear,
        };
        let pending = IconFacts {
            activity: IconActivity::NotRunning,
            attention: IconAttention::Required,
        };
        assert_eq!(icon([Some(running_pending)]), Some(Icon::Running));
        assert_eq!(icon([Some(failed_without_attention)]), Some(Icon::Clear));
        assert_eq!(icon([Some(pending)]), Some(Icon::Attention));
        assert_eq!(
            icon([Some(pending), Some(running_pending)]),
            Some(Icon::Running)
        );
        assert_eq!(
            icon([Some(running_pending), Some(pending)]),
            Some(Icon::Running)
        );
        assert_eq!(icon([Some(running_pending), None]), None);
        assert_eq!(icon([None, Some(pending)]), None);
        assert_eq!(icon([]), Some(Icon::Clear));
    }
}
