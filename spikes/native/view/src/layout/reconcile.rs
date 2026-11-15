//! Hydration is a state, not an empty list: startup gaps cannot erase tabs.
use super::{
    AgentRole, CloseApproval, Layout, LayoutMode, Modified, OpenIntent, Placement, Target,
};
use vessel_proto::{SessionId, TerminalId};

/// Missing entities are authoritative only after their snapshot is hydrated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hydration {
    /// Connection/reconnect has not produced a complete list.
    Pending,
    /// The list is authoritative for this workspace.
    Ready,
}
/// Workspace-owned session facts resolved by the lifecycle controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTab {
    /// Exact session identity.
    pub session: SessionId,
    /// Root auto-opens; children only appear through explicit policy.
    pub role: AgentRole,
}
/// Snapshot is already filtered to this logical workspace, never matched by cwd.
#[derive(Debug)]
pub struct EntitySnapshot<'a> {
    /// Session hydration independent from terminal hydration.
    pub agents: Hydration,
    /// Only nonarchived workspace-owned agents.
    pub sessions: &'a [SessionTab],
    /// Terminal snapshot lifecycle.
    pub terminals: Hydration,
    /// Only standalone terminals; scripts remain owned by their feature.
    pub terminal_ids: &'a [TerminalId],
}
impl Layout {
    /// Apply a whole snapshot atomically; background insertion never steals focus.
    ///
    /// # Errors
    /// Refuses invalid destinations or exhausted local identifiers; changes are atomic.
    pub fn reconcile(
        &mut self,
        snapshot: &EntitySnapshot<'_>,
        mode: LayoutMode,
    ) -> Result<(), super::LayoutError> {
        let mut next = self.clone();
        let stale: Vec<_> = next
            .tabs()
            .filter(|tab| match tab.target() {
                Target::Agent(id) => {
                    snapshot.agents == Hydration::Ready
                        && !snapshot.sessions.iter().any(|entry| entry.session == *id)
                }
                Target::Terminal(id) => {
                    snapshot.terminals == Hydration::Ready && !snapshot.terminal_ids.contains(id)
                }
                Target::NewTab
                | Target::Draft(_)
                | Target::ProviderSubagent { .. }
                | Target::Browser(_)
                | Target::Changes
                | Target::Files
                | Target::Scripts
                | Target::PullRequest
                | Target::File { .. }
                | Target::WorkingDiff
                | Target::Plugin { .. }
                | Target::Setup(_)
                | Target::CommitDiff(_) => false,
            })
            .filter(|tab| tab.modified() == Modified::Clean)
            .map(super::Tab::id)
            .collect();
        for id in stale {
            let plan = next.request_close(id)?;
            next.close_confirmed(&plan, CloseApproval::LifecycleSettled)?;
        }
        if snapshot.agents == Hydration::Ready {
            for entry in snapshot
                .sessions
                .iter()
                .filter(|entry| entry.role == AgentRole::Root)
            {
                let _opened = next.open(
                    Target::Agent(entry.session),
                    OpenIntent::Background,
                    Placement::Focused,
                    entry.role,
                    mode,
                )?;
            }
        }
        if snapshot.terminals == Hydration::Ready {
            for id in snapshot.terminal_ids {
                let _opened = next.open(
                    Target::Terminal(*id),
                    OpenIntent::Background,
                    Placement::Focused,
                    AgentRole::Root,
                    mode,
                )?;
            }
        }
        *self = next;
        Ok(())
    }
}
