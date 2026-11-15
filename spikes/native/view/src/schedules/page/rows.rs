//! Borrowed visible rows: the app receives their badge and available actions,
//! and never makes those decisions while drawing.

use super::super::{Action, Status, authority::Authority};
use super::{Mutation, Page, RowKey};
use vessel_proto::automation::ScheduleInfo;

/// One visible row with its current authority and operation progress.
#[derive(Debug)]
pub struct Row<'a> {
    /// Current admitted connection; cached offline rows cannot offer actions.
    pub connection: Option<&'a crate::connection::ConnectionContext>,
    /// Stable composite identity.
    pub key: RowKey,
    /// Original typed node data, borrowed without copying the prompt.
    pub schedule: &'a ScheduleInfo,
    /// Permission evidence for this row and connection.
    pub authority: &'a Authority,
    /// Row-local operation.
    pub mutation: &'a Mutation,
    /// Host prefix is present only for a fleet with multiple machines.
    pub host: Option<super::super::load::MachineKey>,
    /// Target lookup, read only when this list contains worktree targets.
    pub worktrees:
        &'a Result<Vec<vessel_proto::workspace::WorktreeInfo>, super::super::load::Failure>,
    /// Project names read for this list, independent of another screen's cache.
    pub projects:
        &'a Result<Vec<vessel_proto::workspace::ProjectInfo>, super::super::load::Failure>,
}

/// One row message: a refusal is shown once, even when it also established
/// the row's authority for this connection.
#[derive(Debug, PartialEq, Eq)]
pub enum Feedback<'a> {
    /// Device may only read this row's schedule.
    ReadOnly,
    /// Another operation failed.
    Failed(&'a super::super::load::Failure),
}

impl Feedback<'_> {
    /// Shared Alert meaning of this row's single message.
    #[must_use]
    pub const fn variant(&self) -> crate::alert::Variant {
        match self {
            Self::ReadOnly => crate::alert::Variant::Info,
            Self::Failed(failure) => failure.variant(),
        }
    }
}

impl Row<'_> {
    /// Row-local feedback, with authority evidence taking precedence over the
    /// identical Forbidden failure so the person never sees the line twice.
    #[must_use]
    pub fn feedback(&self) -> Option<Feedback<'_>> {
        if !self.authority.allows_actions() {
            return Some(Feedback::ReadOnly);
        }
        self.mutation.failure().map(Feedback::Failed)
    }
    /// Wired actions in reference order; editing opens the pure form owner.
    #[must_use]
    pub fn actions(&self) -> Vec<Action> {
        if self.connection.is_none() || self.mutation.blocks_actions() {
            return Vec::new();
        }
        super::super::actions(self.schedule, self.authority)
    }
}

impl Page {
    /// Find one row even if its filter changed while its menu was open.
    #[must_use]
    pub fn row(&self, key: &RowKey) -> Option<Row<'_>> {
        let cache = self.caches.get(&key.machine)?;
        let snapshot = cache.read.snapshot()?;
        let schedule = snapshot
            .schedules
            .iter()
            .find(|info| info.id == key.schedule)?;
        let state = cache.rows.get(&key.schedule)?;
        Some(Row {
            connection: cache.connection.as_ref(),
            key: key.clone(),
            schedule,
            authority: &state.authority,
            mutation: &state.mutation,
            host: self.multiple_machines().then(|| key.machine.clone()),
            worktrees: &snapshot.worktrees,
            projects: &snapshot.projects,
        })
    }

    /// The reference's stable newest-first order, scoped to host/status filters.
    #[must_use]
    pub fn visible_rows(&self) -> Vec<Row<'_>> {
        let mut rows = Vec::new();
        for (machine, cache) in &self.caches {
            if !self.selected(machine) {
                continue;
            }
            let Some(snapshot) = cache.read.snapshot() else {
                continue;
            };
            for info in &snapshot.schedules {
                if Status::of(&info.state).bucket() != self.bucket {
                    continue;
                }
                if let Some(row) = self.row(&RowKey {
                    machine: machine.clone(),
                    schedule: info.id,
                }) {
                    rows.push(row);
                }
            }
        }
        rows.sort_by(|left, right| {
            right
                .schedule
                .created_at
                .cmp(&left.schedule.created_at)
                .then_with(|| left.key.cmp(&right.key))
        });
        rows
    }

    /// A newer machine's rows remain reachable in a separate filter.
    #[must_use]
    pub fn buckets(&self) -> Vec<super::super::Bucket> {
        use super::super::Bucket;
        let mut buckets = vec![Bucket::Active, Bucket::Ended];
        if self
            .caches
            .values()
            .filter_map(|cache| cache.read.snapshot())
            .flat_map(|snapshot| &snapshot.schedules)
            .any(|info| Status::of(&info.state).bucket() == Bucket::Unknown)
        {
            buckets.push(Bucket::Unknown);
        }
        buckets
    }
}

/// How a visible action may respond to input; an operation remains in its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    /// Current node state and authority permit activation.
    Enabled,
    /// Another row-local operation is in flight.
    Disabled,
    /// This exact action is in flight, with its own translated pending label.
    Pending,
}
/// One visible, wired list action in the reference's menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuEntry {
    /// Typed action, including the original direction if a poll races its reply.
    pub action: Action,
    /// Input/pending meaning only; the app passes it to the shared menu.
    pub state: EntryState,
}
impl Row<'_> {
    /// A node poll may reveal Pause before the Pause RPC settles. Keep Pause
    /// pending in the cadence slot instead of incorrectly saying Resuming.
    #[must_use]
    pub fn menu_entries(&self) -> Vec<MenuEntry> {
        if self.connection.is_none()
            || !self.authority.allows_actions()
            || matches!(self.mutation, Mutation::Unconfirmed { .. })
        {
            return Vec::new();
        }
        let allowed = super::super::actions(self.schedule, self.authority);
        let pending = self.mutation.progress();
        [
            Action::Edit,
            Action::Pause,
            Action::Resume,
            Action::RunNow,
            Action::Delete,
        ]
        .into_iter()
        .filter(|action| {
            // Paseo's cadence actions belong only to a new-chat target.
            // Existing-session node operations remain available to typed
            // callers; the heartbeat menu exposes only Edit and Delete.
            if matches!(
                self.schedule.definition.target,
                vessel_proto::automation::ScheduleTarget::Session { .. }
            ) && matches!(action, Action::Pause | Action::Resume | Action::RunNow)
            {
                return false;
            }
            let opposite = match pending {
                Some(Action::Pause) => *action == Action::Resume,
                Some(Action::Resume) => *action == Action::Pause,
                Some(Action::RunNow | Action::Delete | Action::Edit) | None => false,
            };
            !opposite && (allowed.contains(action) || pending == Some(*action))
        })
        .map(|action| MenuEntry {
            action,
            state: match pending {
                None => EntryState::Enabled,
                Some(active) => {
                    if active == action {
                        EntryState::Pending
                    } else {
                        EntryState::Disabled
                    }
                }
            },
        })
        .collect()
    }
}
