//! A track keeps managed addresses and provider tasks in separate typed rows.

use crate::devices::{MachineKey, Ticket, Tickets};
use std::collections::HashSet;
use vessel_proto::subagents::{
    ChildDescriptor, ChildHistory, ChildStatus, ProviderChildId, ProviderChildren,
};
use vessel_proto::{ErrorCode, NodeId, SessionInfo, SessionRef};

/// Authenticated machine identity plus its parsed book route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    /// Routing label; identity is pinned separately.
    pub machine: MachineKey,
    /// Node that answered the welcome.
    pub node: NodeId,
}
/// A managed session target is never a provider child id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Routing label; changing its destination cannot change the address.
    pub machine: MachineKey,
    /// Complete canonical node/session identity.
    pub address: SessionRef,
}
/// Typed failures retain uncertainty for a detach whose response was lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TrackProblem {
    /// Read cannot reach the host.
    #[error("host is offline")]
    Offline,
    /// The node has no assigned/capable relationship protocol.
    #[error("subagent relationships are unsupported")]
    Unsupported,
    /// A book alias now points at another authenticated node.
    #[error("host identity changed")]
    IdentityChanged,
    /// Read deadline elapsed.
    #[error("subagent read timed out")]
    Timeout,
    /// Mutation may have committed; refresh before another action.
    #[error("detach result is uncertain")]
    Uncertain,
    /// Structurally invalid or over-budget reply.
    #[error("subagent reply is invalid")]
    InvalidReply,
    /// Correlation identities cannot be reused.
    #[error("subagent request identities exhausted")]
    Exhausted,
    /// An existing request must settle before another page is issued.
    #[error("subagent request is already running")]
    Busy,
    /// Earlier pages would exceed this pane's finite retained-history budget.
    #[error("provider history reached its retention limit")]
    RetentionLimit,
    /// The node's typed refusal; no raw provider error is shown.
    #[error("node refused the subagent request: {0:?}")]
    Refused(ErrorCode),
}
/// Current managed child facts. A2 supplies authoritative finished eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedChild {
    /// Opening/detaching routes to this complete address.
    pub target: Target,
    /// Original server metadata, never reconstructed from a title or cwd.
    pub info: Box<SessionInfo>,
    /// Canonical conditional-archive eligibility from A2, not display wording.
    pub finished: Option<bool>,
    /// Pending archive/background filtering comes from the owning registry/action.
    pub visibility: super::Visibility,
}
/// A request keeps its original parent and generation across navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRead {
    /// Pinned managed parent.
    pub target: Target,
    /// Reject stale replies even for a reopened parent.
    pub ticket: Ticket,
}
/// A detach confirmation captures the child and parent it was offered for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachChoice {
    /// Parent whose current track offered the action.
    pub parent: SessionRef,
    /// Exact managed child; no provider task can inhabit this type.
    pub child: Target,
}
/// Read state is independent from retained good rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrackLoad {
    /// Nothing outstanding.
    #[default]
    Idle,
    /// A specific request is outstanding.
    Loading(Ticket),
    /// Previous good rows remain visible.
    Failed(TrackProblem),
}
/// Prepared counts for the composer pill, without inferring unknown statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrackCounts {
    /// Visible managed and provider children.
    pub total: usize,
    /// Providers explicitly reporting running; managed work is painted separately.
    pub running: usize,
    /// Canonically eligible managed rows plus terminal provider statuses.
    pub finished: usize,
}

/// Active source status groups, in the same failed-before-working order as Paseo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    /// An explicit managed/provider failure.
    Failed,
    /// An actual active managed turn or running provider task.
    Working,
}
/// Each status keeps its own count; a mixed fan-out never adopts one urgent state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PillCount {
    /// Status whose mark accompanies this count.
    pub kind: PillKind,
    /// Exact visible root-row count in this group.
    pub count: usize,
}

/// One parent's track; provider hide is local and never archives/stops anything.
#[derive(Debug)]
pub struct Track {
    target: Target,
    managed: Vec<ManagedChild>,
    providers: Vec<ChildDescriptor>,
    hidden: HashSet<ProviderChildId>,
    load: TrackLoad,
    coverage: ChildHistory,
}
impl Track {
    /// Start with unknown provider history, not an invented empty success.
    #[must_use]
    pub fn new(target: Target) -> Self {
        Self {
            target,
            managed: Vec::new(),
            providers: Vec::new(),
            hidden: HashSet::new(),
            load: TrackLoad::Idle,
            coverage: ChildHistory::Missing,
        }
    }
    /// The parent identity cannot change behind an in-flight action.
    #[must_use]
    pub fn target(&self) -> &Target {
        &self.target
    }
    /// Oldest-first managed children, already filtered by current parentage.
    #[must_use]
    pub fn managed(&self) -> &[ManagedChild] {
        &self.managed
    }
    /// Visible provider nesting under the requested real provider parent.
    pub fn providers<'a>(
        &'a self,
        parent: Option<&'a ProviderChildId>,
    ) -> impl Iterator<Item = &'a ChildDescriptor> {
        self.providers
            .iter()
            .filter(move |row| self.visible_under(row, parent) && !self.hidden.contains(&row.id))
    }
    /// Resolve an observed descriptor without inventing missing parent metadata.
    #[must_use]
    pub fn provider(&self, id: &ProviderChildId) -> Option<&ChildDescriptor> {
        self.providers.iter().find(|row| &row.id == id)
    }
    /// Ancestors come from the same validated bounded descriptor set as rows.
    #[must_use]
    pub fn provider_path(&self, id: &ProviderChildId) -> Vec<&ChildDescriptor> {
        let mut path = Vec::new();
        let mut next = self.provider(id);
        while let Some(row) = next {
            path.push(row);
            if path.len() >= vessel_proto::subagents::bounds::MAX_CHILD_DEPTH {
                break;
            }
            next = row.parent.as_ref().and_then(|id| self.provider(id));
        }
        path.reverse();
        path
    }
    fn visible_under(&self, row: &ChildDescriptor, parent: Option<&ProviderChildId>) -> bool {
        match parent {
            Some(parent) => row.parent.as_ref() == Some(parent),
            None => row.parent.as_ref().is_none_or(|parent| {
                !self
                    .providers
                    .iter()
                    .any(|candidate| &candidate.id == parent)
            }),
        }
    }
    /// Current request/failure without discarding good data.
    #[must_use]
    pub const fn load(&self) -> TrackLoad {
        self.load
    }
    /// Complete/partial/missing/unsupported never collapse to a guessed empty list.
    #[must_use]
    pub const fn coverage(&self) -> ChildHistory {
        self.coverage
    }
    /// Reserve a refresh from the owner's navigation-lifetime ticket source.
    /// # Errors
    /// Correlation counter exhausted.
    pub fn refresh(&mut self, tickets: &mut Tickets) -> Result<TrackRead, TrackProblem> {
        let ticket = tickets
            .next_ticket()
            .map_err(|_error| TrackProblem::Exhausted)?;
        self.load = TrackLoad::Loading(ticket);
        Ok(TrackRead {
            target: self.target.clone(),
            ticket,
        })
    }
    /// A stale read cannot alter another parent or its hide state.
    pub fn listed(
        &mut self,
        read: &TrackRead,
        result: Result<ProviderChildren, TrackProblem>,
    ) -> bool {
        if read.target != self.target || self.load != TrackLoad::Loading(read.ticket) {
            return false;
        }
        match result {
            Ok(rows) => {
                self.providers = rows.children.as_slice().to_vec();
                self.coverage = rows.history;
                self.load = TrackLoad::Idle;
            }
            Err(problem) => self.load = TrackLoad::Failed(problem),
        }
        true
    }
    /// Replace one host's hydrated managed rows while preserving other hosts.
    /// # Errors
    /// Corrupt current parentage or excessive snapshots refuse the update.
    pub fn managed_host(
        &mut self,
        host: &Host,
        rows: Vec<ManagedChild>,
    ) -> Result<(), TrackProblem> {
        if rows.len() > super::MAX_MANAGED_SNAPSHOTS {
            return Err(TrackProblem::InvalidReply);
        }
        let mut retained = self
            .managed
            .iter()
            .filter(|row| row.target.address.node != host.node)
            .cloned()
            .collect::<Vec<_>>();
        for row in rows {
            if row.target.address.node != host.node
                || row.target.machine != host.machine
                || row.info.id != row.target.address.session
            {
                return Err(TrackProblem::IdentityChanged);
            }
            let parent = row
                .info
                .parentage()
                .map_err(|_error| TrackProblem::InvalidReply)?
                .parent();
            if parent == Some(self.target.address)
                && row.info.marks.archived_at.is_none()
                && row.visibility == super::Visibility::Active
            {
                retained.push(row);
            }
        }
        if retained.len() > super::MAX_MANAGED_SNAPSHOTS {
            return Err(TrackProblem::InvalidReply);
        }
        retained.sort_by_key(|row| row.info.created_at);
        let mut seen = HashSet::new();
        if retained.iter().any(|row| !seen.insert(row.target.address)) {
            return Err(TrackProblem::InvalidReply);
        }
        self.managed = retained;
        Ok(())
    }
    /// Confirm only a currently attached managed child; providers have no detach.
    #[must_use]
    pub fn detach_choice(&self, child: SessionRef) -> Option<DetachChoice> {
        self.managed
            .iter()
            .find(|row| row.target.address == child)
            .map(|row| DetachChoice {
                parent: self.target.address,
                child: row.target.clone(),
            })
    }
    /// Local hide is scoped by this parent and bounded by the node descriptor cap.
    pub fn hide_provider(&mut self, id: &ProviderChildId) -> bool {
        if !self.providers.iter().any(|row| &row.id == id)
            || self.hidden.len() >= vessel_proto::subagents::bounds::MAX_CHILDREN
        {
            return false;
        }
        self.hidden.insert(id.clone())
    }
    /// Terminal provider rows are hidden locally; managed archival is A2's call.
    pub fn hide_finished_providers(&mut self) {
        let ids = self
            .providers
            .iter()
            .filter(|row| self.visible_under(row, None) && finished_provider(row.status))
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            self.hide_provider(&id);
        }
    }
    /// Unknown, idle and completed children contribute no invented active mark.
    #[must_use]
    pub fn pill_counts(&self) -> Vec<PillCount> {
        use vessel_proto::{Activity, SessionStatus};
        let mut failed = 0;
        let mut working = 0;
        for row in &self.managed {
            if matches!(row.info.status, SessionStatus::Failed { .. }) {
                failed += 1;
            } else if matches!(row.info.status, SessionStatus::Running)
                && row
                    .info
                    .board
                    .as_ref()
                    .is_some_and(|board| board.activity == Activity::Active)
            {
                working += 1;
            }
        }
        for row in self.providers(None) {
            match row.status {
                Some(ChildStatus::Failed) => failed += 1,
                Some(ChildStatus::Running) => working += 1,
                Some(ChildStatus::Completed | ChildStatus::Canceled) | None => {}
            }
        }
        [
            PillCount {
                kind: PillKind::Failed,
                count: failed,
            },
            PillCount {
                kind: PillKind::Working,
                count: working,
            },
        ]
        .into_iter()
        .filter(|segment| segment.count > 0)
        .collect()
    }
    /// Counts come from exactly the same visible rows as navigation.
    #[must_use]
    pub fn counts(&self) -> TrackCounts {
        let providers = self
            .providers
            .iter()
            .filter(|row| self.visible_under(row, None) && !self.hidden.contains(&row.id))
            .collect::<Vec<_>>();
        TrackCounts {
            total: self.managed.len() + providers.len(),
            running: providers
                .iter()
                .filter(|row| row.status == Some(ChildStatus::Running))
                .count(),
            finished: self
                .managed
                .iter()
                .filter(|row| row.finished == Some(true))
                .count()
                + providers
                    .iter()
                    .filter(|row| self.visible_under(row, None) && finished_provider(row.status))
                    .count(),
        }
    }
}
fn finished_provider(status: Option<ChildStatus>) -> bool {
    matches!(
        status,
        Some(ChildStatus::Completed | ChildStatus::Failed | ChildStatus::Canceled)
    )
}

#[cfg(test)]
mod tests;
