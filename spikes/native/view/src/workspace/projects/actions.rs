//! Host-bound project mutations: cancelled forms never accept an older completion.
//! The node remains authoritative; correlation only controls which draft settles.

use crate::machine_book::Name;
use vessel_proto::{
    NodeId, ProjectId,
    workspace::{AddProject, CloneProject, InitProject, ProjectInfo},
};

/// A request identity survives switching hosts, cancelling and reopening the same form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectActionKey {
    host: Name,
    node: NodeId,
    serial: u64,
}
impl ProjectActionKey {
    /// The validated node-book target captured when the person submitted.
    #[must_use]
    pub const fn host(&self) -> &Name {
        &self.host
    }
    /// Exact authenticated welcome identity; an alias alone cannot authorize a mutation.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
}

/// Existing project RPCs only; new directory-without-Git has its own pending node contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectAction {
    /// Register an existing folder without creating a workspace/session.
    Register(AddProject),
    /// Clone on the selected host, following the returned project afterward.
    Clone(CloneProject),
    /// Explicit Git initialization remains distinct from an empty new directory.
    Initialize(InitProject),
    /// Forget a concrete host-local project after confirmation.
    Remove(ProjectId),
}

/// Typed result echoes the operation, rather than interpreting a successful unrelated reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectChanged {
    /// Registration/clone/init returned this authoritative snapshot.
    Registered(ProjectInfo),
    /// Only the named project was removed from registry membership.
    Removed(ProjectId),
}

/// An unbounded serial must refuse exhaustion instead of making a stale ticket current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProjectActionError {
    /// A single form keeps one outstanding request; repeated submission is a no-op at UI.
    #[error("a project operation is already running")]
    Busy,
    /// A process restart is needed before another unique ticket can be generated.
    #[error("project operation identifiers are exhausted")]
    Exhausted,
    /// Detached remote calls remain tracked; repeated cancellation cannot grow memory forever.
    #[error("too many project operations still need reconciliation")]
    ReconciliationFull,
}

/// Readiness of manual project work for a fixed authenticated host.
/// This says nothing about a suggested-project checked-set acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectSetupReadiness {
    /// All observed manual work settled successfully.
    Ready,
    /// A request or a clone is still running, even if its form was closed.
    Pending,
    /// An observed operation failed and requires explicit reconciliation or retry.
    Failed,
}

/// Onboarding's decision from a current admitted list and settled manual work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectInventory {
    /// The current list or manual operation has not settled.
    Pending,
    /// Admission/read/manual work failed or the empty list is incomplete.
    Unavailable,
    /// A complete authoritative list contains no projects.
    Empty,
    /// The admitted node has at least one project.
    Existing,
}

/// A transport result settles tracking independently of which form is now visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectSettlement {
    /// The node returned an authoritative success.
    Succeeded,
    /// Refused, timed out or failed; the app must not infer remote rollback.
    Failed,
}

const MAX_PROJECT_RECONCILIATIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivityState {
    Pending,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Activity {
    key: ProjectActionKey,
    state: ActivityState,
}

/// Explicit disposition prevents a stale response from closing a newer form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectReply {
    /// This response owns the current form's in-flight state.
    Current,
    /// Re-read current authority; this payload cannot hydrate a newer host binding or draft.
    Stale,
}

/// State is pure and owned by one screen; transport work stays in link.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectActions {
    serial: u64,
    pending: Option<ProjectActionKey>,
    activities: Vec<Activity>,
}
impl ProjectActions {
    /// Capture the chosen host before a node call is submitted.
    ///
    /// # Errors
    /// A pending action or exhausted identifier space refuses duplicate dispatch.
    pub fn begin(
        &mut self,
        host: Name,
        node: NodeId,
    ) -> Result<ProjectActionKey, ProjectActionError> {
        if self.pending.is_some() {
            return Err(ProjectActionError::Busy);
        }
        let serial = self
            .serial
            .checked_add(1)
            .ok_or(ProjectActionError::Exhausted)?;
        self.activities.retain(|activity| {
            !(activity.state == ActivityState::Failed
                && activity.key.host == host
                && activity.key.node == node)
        });
        if self.activities.len() >= MAX_PROJECT_RECONCILIATIONS {
            return Err(ProjectActionError::ReconciliationFull);
        }
        self.serial = serial;
        let key = ProjectActionKey {
            host,
            node,
            serial: self.serial,
        };
        self.pending = Some(key.clone());
        self.activities.push(Activity {
            key: key.clone(),
            state: ActivityState::Pending,
        });
        Ok(key)
    }
    /// Cancelling detaches the draft, without claiming the remote side effect was undone.
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    /// Every reply settles remote activity even after its form was detached.
    #[must_use]
    pub fn finish(&mut self, key: &ProjectActionKey, result: ProjectSettlement) -> ProjectReply {
        match result {
            ProjectSettlement::Succeeded => self.activities.retain(|activity| &activity.key != key),
            ProjectSettlement::Failed => {
                if let Some(activity) = self
                    .activities
                    .iter_mut()
                    .find(|activity| &activity.key == key)
                {
                    activity.state = ActivityState::Failed;
                }
            }
        }
        if self.pending.as_ref() == Some(key) {
            self.pending = None;
            ProjectReply::Current
        } else {
            ProjectReply::Stale
        }
    }
    /// Rendering can disable submit without holding a second busy flag.
    #[must_use]
    pub const fn pending(&self) -> Option<&ProjectActionKey> {
        self.pending.as_ref()
    }

    /// Query exactly one Name+NodeId; aliases reused by another node cannot inherit readiness.
    #[must_use]
    pub fn setup_readiness(&self, host: &Name, node: NodeId) -> ProjectSetupReadiness {
        let mut result = ProjectSetupReadiness::Ready;
        for activity in &self.activities {
            if &activity.key.host == host && activity.key.node == node {
                if activity.state == ActivityState::Pending {
                    return ProjectSetupReadiness::Pending;
                }
                result = ProjectSetupReadiness::Failed;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn host(name: &str) -> Name {
        Name::parse(name).unwrap()
    }
    #[test]
    fn repeated_submission_does_not_allocate_or_dispatch_another_action() {
        let mut state = ProjectActions::default();
        let key = state.begin(host("laptop"), NodeId::random()).unwrap();
        assert_eq!(
            state.begin(host("laptop"), NodeId::random()),
            Err(ProjectActionError::Busy)
        );
        assert_eq!(state.pending(), Some(&key));
    }
    #[test]
    fn cancelled_completion_cannot_close_a_new_form_on_the_same_host() {
        let mut state = ProjectActions::default();
        let old = state.begin(host("laptop"), NodeId::random()).unwrap();
        state.cancel();
        let new = state.begin(host("laptop"), NodeId::random()).unwrap();
        assert_eq!(
            state.finish(&old, ProjectSettlement::Succeeded),
            ProjectReply::Stale
        );
        assert_eq!(state.pending(), Some(&new));
        assert_eq!(
            state.finish(&new, ProjectSettlement::Succeeded),
            ProjectReply::Current
        );
    }
    #[test]
    fn switching_hosts_never_settles_the_other_hosts_form() {
        let mut state = ProjectActions::default();
        let old = state.begin(host("laptop"), NodeId::random()).unwrap();
        state.cancel();
        let new = state.begin(host("build-server"), NodeId::random()).unwrap();
        assert_eq!(
            state.finish(&old, ProjectSettlement::Succeeded),
            ProjectReply::Stale
        );
        assert_eq!(new.host().as_str(), "build-server");
        assert_eq!(state.pending(), Some(&new));
    }
    #[test]
    fn identifier_exhaustion_never_wraps() {
        let mut state = ProjectActions {
            serial: u64::MAX,
            pending: None,
            activities: Vec::new(),
        };
        assert_eq!(
            state.begin(host("laptop"), NodeId::random()),
            Err(ProjectActionError::Exhausted)
        );
        assert!(state.pending().is_none());
    }

    #[test]
    fn identifier_exhaustion_does_not_erase_an_unreconciled_failure() {
        let mut state = ProjectActions::default();
        let host = host("laptop");
        let node = NodeId::random();
        let key = state.begin(host.clone(), node).unwrap();
        assert_eq!(
            state.finish(&key, ProjectSettlement::Failed),
            ProjectReply::Current
        );
        state.serial = u64::MAX;
        assert_eq!(
            state.begin(host.clone(), node),
            Err(ProjectActionError::Exhausted)
        );
        assert_eq!(
            state.setup_readiness(&host, node),
            ProjectSetupReadiness::Failed
        );
    }

    #[test]
    fn closing_a_form_keeps_its_remote_work_pending_until_a_matching_reply() {
        let mut state = ProjectActions::default();
        let host = host("laptop");
        let node = NodeId::random();
        let key = state.begin(host.clone(), node).unwrap();
        state.cancel();
        assert_eq!(
            state.setup_readiness(&host, node),
            ProjectSetupReadiness::Pending
        );
        assert_eq!(
            state.setup_readiness(&host, NodeId::random()),
            ProjectSetupReadiness::Ready
        );
        assert_eq!(
            state.finish(&key, ProjectSettlement::Failed),
            ProjectReply::Stale
        );
        assert_eq!(
            state.setup_readiness(&host, node),
            ProjectSetupReadiness::Failed
        );
        let retry = state.begin(host.clone(), node).unwrap();
        assert_eq!(
            state.setup_readiness(&host, node),
            ProjectSetupReadiness::Pending
        );
        assert_eq!(
            state.finish(&retry, ProjectSettlement::Succeeded),
            ProjectReply::Current
        );
        assert_eq!(
            state.setup_readiness(&host, node),
            ProjectSetupReadiness::Ready
        );
    }

    #[test]
    fn detached_requests_have_a_bound_and_can_settle_at_capacity() {
        let mut state = ProjectActions::default();
        let host = host("laptop");
        let node = NodeId::random();
        let mut keys = Vec::new();
        for _ in 0..MAX_PROJECT_RECONCILIATIONS {
            keys.push(state.begin(host.clone(), node).unwrap());
            state.cancel();
        }
        assert_eq!(
            state.begin(host.clone(), node),
            Err(ProjectActionError::ReconciliationFull)
        );
        assert_eq!(
            state.finish(&keys[0], ProjectSettlement::Succeeded),
            ProjectReply::Stale
        );
        assert!(state.begin(host, node).is_ok());
    }
}
