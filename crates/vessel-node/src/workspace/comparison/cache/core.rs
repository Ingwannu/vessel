//! Completed and active comparison ownership after checkout-diff-cache.ts.
//! The runtime actor owns this value; no cache mutex or subprocess lives here.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{CompareGit, GitComparison, WorkspaceRoot, WorktreeBase};
use vessel_proto::workspace::{GitHunkOffset, RelPath};
use vessel_workspace::paths::Root;

/// Completed payloads, independently of in-flight reads.
pub(in crate::workspace::comparison) const MAX_COMPLETED: usize = 64;
/// A bounded admission floor in addition to the runtime's four executing jobs.
pub(in crate::workspace::comparison) const MAX_ACTIVE: usize = 64;
pub(in crate::workspace::comparison) const FRESH_FOR: Duration = Duration::from_secs(15);
pub(in crate::workspace::comparison) const MIN_READ_GAP: Duration = Duration::from_secs(2);

/// Root resolution/authorization must precede construction, even on a hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace::comparison) struct Key {
    pub(in crate::workspace::comparison) canonical: Root,
    /// Same resolved authority snapshot as canonical/query; never inferred from paths.
    pub(in crate::workspace::comparison) project: ProjectId,
    pub(in crate::workspace::comparison) query: CompareGit,
    pub(in crate::workspace::comparison) subject: Subject,
}

impl Key {
    #[must_use]
    pub(in crate::workspace::comparison) fn new(
        project: ProjectId,
        canonical: Root,
        mut query: CompareGit,
        default: Option<WorktreeBase>,
        subject: Subject,
    ) -> Self {
        match &mut query.comparison {
            GitComparison::Committed { base } | GitComparison::All { base } => {
                if base.is_none() {
                    *base = default;
                }
            }
            GitComparison::Uncommitted | GitComparison::Commit { .. } => {}
        }
        query.reason = vessel_proto::workspace::GitReadReason::View;
        Self {
            canonical,
            project,
            query,
            subject,
        }
    }
}

/// The existing lazy file API retains its page identity in the shared bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace::comparison) enum Subject {
    Summary,
    File {
        path: RelPath,
        offset: GitHunkOffset,
    },
}

/// Force cannot be requested without naming why it bypasses freshness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace::comparison) enum ForceReason {
    Person,
    Mutation,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace::comparison) enum ReadReason {
    View,
    Force(ForceReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace) enum Change {
    Working,
    History,
    Repository,
}

impl Change {
    fn affects(self, comparison: &GitComparison) -> bool {
        match (self, comparison) {
            (
                Self::Repository,
                GitComparison::Uncommitted
                | GitComparison::Committed { .. }
                | GitComparison::All { .. }
                | GitComparison::Commit { .. },
            )
            | (Self::Working, GitComparison::Uncommitted | GitComparison::All { .. })
            | (Self::History, GitComparison::Committed { .. } | GitComparison::All { .. }) => true,
            (Self::Working, GitComparison::Committed { .. } | GitComparison::Commit { .. })
            | (Self::History, GitComparison::Uncommitted | GitComparison::Commit { .. }) => false,
        }
    }
}

impl From<vessel_proto::workspace::GitReadReason> for ReadReason {
    fn from(reason: vessel_proto::workspace::GitReadReason) -> Self {
        use vessel_proto::workspace::GitReadReason;
        match reason {
            GitReadReason::View => Self::View,
            GitReadReason::Refresh => Self::Force(ForceReason::Person),
            GitReadReason::Mutation => Self::Force(ForceReason::Mutation),
            GitReadReason::Recovery => Self::Force(ForceReason::Recovery),
        }
    }
}

/// An actor-lifetime ticket, never reused after an invalidated read finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace::comparison) struct BuildId(u64);

#[derive(Debug)]
pub(in crate::workspace::comparison) enum Read<V> {
    Hit(Arc<V>),
    Start(BuildId),
    Join(BuildId),
    After(BuildId),
}

#[derive(Debug)]
pub(in crate::workspace::comparison) struct Decision<V> {
    pub(in crate::workspace::comparison) read: Read<V>,
    /// Cancel the owned work, never abort its cleanup task.
    pub(in crate::workspace::comparison) cancel: Option<BuildId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Refusal {
    #[error("too many comparison reads are active")]
    Busy,
    #[error("comparison read identifiers are exhausted")]
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveState {
    Current,
    Superseded,
}

#[derive(Debug)]
struct Active {
    key: Key,
    id: BuildId,
    state: ActiveState,
    started: Instant,
}

#[derive(Debug)]
struct Completed<V> {
    key: Key,
    value: Arc<V>,
    loaded: Instant,
    started: Instant,
}

#[derive(Debug)]
pub(in crate::workspace::comparison) enum ResultValue<V> {
    Value(Arc<V>),
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace::comparison) enum Completion {
    Published,
    Retired,
    Failed,
    Unknown,
}

#[derive(Debug)]
pub(in crate::workspace::comparison) struct Cache<V> {
    completed: VecDeque<Completed<V>>,
    active: VecDeque<Active>,
    issued: u64,
}

impl<V> Default for Cache<V> {
    fn default() -> Self {
        Self {
            completed: VecDeque::new(),
            active: VecDeque::new(),
            issued: 0,
        }
    }
}

impl<V> Cache<V> {
    /// The actor starts work only after receiving Start; active ownership is
    /// installed first, including a synchronous invalidation during setup.
    pub(in crate::workspace::comparison) fn read(
        &mut self,
        key: Key,
        reason: ReadReason,
        now: Instant,
    ) -> Result<Decision<V>, Refusal> {
        if let Some(cached) = self.touch(&key) {
            match reason {
                ReadReason::View
                    if now.saturating_duration_since(cached.loaded) <= FRESH_FOR
                        || now.saturating_duration_since(cached.started) < MIN_READ_GAP =>
                {
                    return Ok(Decision {
                        read: Read::Hit(Arc::clone(&cached.value)),
                        cancel: None,
                    });
                }
                ReadReason::View | ReadReason::Force(_) => {}
            }
        }
        if let Some(decision) = self.pending(&key, reason) {
            return Ok(decision);
        }
        if self.active.len() >= MAX_ACTIVE {
            return Err(Refusal::Busy);
        }
        let issued = self.issued.checked_add(1).ok_or(Refusal::Exhausted)?;
        self.issued = issued;
        if let Some(cached) = self.touch(&key) {
            cached.started = now;
        }
        let id = BuildId(issued);
        self.active.push_back(Active {
            key,
            id,
            state: ActiveState::Current,
            started: now,
        });
        Ok(Decision {
            read: Read::Start(id),
            cancel: None,
        })
    }

    fn pending(&mut self, key: &Key, reason: ReadReason) -> Option<Decision<V>> {
        if let Some(active) = self.active.iter_mut().find(|active| active.key == *key) {
            let cancel = match reason {
                ReadReason::Force(_) if active.state == ActiveState::Current => {
                    active.state = ActiveState::Superseded;
                    self.completed.retain(|cached| cached.key != *key);
                    Some(active.id)
                }
                ReadReason::View | ReadReason::Force(_) => None,
            };
            let read = match active.state {
                ActiveState::Current => Read::Join(active.id),
                ActiveState::Superseded => Read::After(active.id),
            };
            return Some(Decision { read, cancel });
        }
        None
    }

    /// Original callers receive their own result independently. A superseded
    /// result may never refill this cache; fresh waiters retry only after the
    /// runtime has completed and cleaned the old job.
    pub(in crate::workspace::comparison) fn finish(
        &mut self,
        id: BuildId,
        value: ResultValue<V>,
        now: Instant,
    ) -> Completion {
        let Some(index) = self.active.iter().position(|active| active.id == id) else {
            return Completion::Unknown;
        };
        let Some(active) = self.active.remove(index) else {
            return Completion::Unknown;
        };
        match (active.state, value) {
            (ActiveState::Current, ResultValue::Value(value)) => {
                self.completed.retain(|cached| cached.key != active.key);
                self.completed.push_front(Completed {
                    key: active.key.clone(),
                    value,
                    loaded: now,
                    started: active.started,
                });
                self.completed.truncate(MAX_COMPLETED);
                Completion::Published
            }
            (ActiveState::Superseded, ResultValue::Value(_)) => Completion::Retired,
            (ActiveState::Current | ActiveState::Superseded, ResultValue::Failed) => {
                Completion::Failed
            }
        }
    }

    pub(in crate::workspace::comparison) fn invalidate(
        &mut self,
        root: WorkspaceRoot,
        project: ProjectId,
        change: Change,
    ) -> Vec<BuildId> {
        // Ref writes affect every registered sibling worktree of this project.
        // Index/worktree writes remain local. This is cache invalidation only;
        // it never resolves a repository or grants another root's authority.
        let affected = |key: &Key| {
            let owner = match change {
                Change::Working => key.query.root == root,
                Change::History | Change::Repository => key.project == project,
            };
            owner && change.affects(&key.query.comparison)
        };
        self.completed.retain(|cached| !affected(&cached.key));
        self.active
            .iter_mut()
            .filter_map(|active| {
                if affected(&active.key) && active.state == ActiveState::Current {
                    active.state = ActiveState::Superseded;
                    Some(active.id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// A forced scope refresh also retires its lazy sibling pages. The current
    /// part is handled by `read()`, so concurrent waiters share its retry.
    pub(in crate::workspace::comparison) fn invalidate_siblings(
        &mut self,
        key: &Key,
    ) -> Vec<BuildId> {
        let sibling = |candidate: &Key| {
            candidate.canonical == key.canonical
                && candidate.query == key.query
                && candidate.subject != key.subject
        };
        self.completed.retain(|cached| !sibling(&cached.key));
        self.active
            .iter_mut()
            .filter_map(|active| {
                if sibling(&active.key) && active.state == ActiveState::Current {
                    active.state = ActiveState::Superseded;
                    Some(active.id)
                } else {
                    None
                }
            })
            .collect()
    }

    fn touch(&mut self, key: &Key) -> Option<&mut Completed<V>> {
        let index = self
            .completed
            .iter()
            .position(|cached| cached.key == *key)?;
        let cached = self.completed.remove(index)?;
        self.completed.push_front(cached);
        self.completed.front_mut()
    }
}

#[cfg(test)]
impl<T> Cache<T> {
    pub(in crate::workspace::comparison) fn completed_for_test(&self) -> usize {
        self.completed.len()
    }
}

#[cfg(test)]
mod tests;
