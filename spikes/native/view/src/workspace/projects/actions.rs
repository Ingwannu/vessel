//! Host-bound project mutations: cancelled forms never accept an older completion.
//! The node remains authoritative; correlation only controls which draft settles.

use crate::machine_book::Name;
use vessel_proto::{
    ProjectId,
    workspace::{AddProject, CloneProject, InitProject, ProjectInfo},
};

/// A request identity survives switching hosts, cancelling and reopening the same form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectActionKey {
    host: Name,
    serial: u64,
}
impl ProjectActionKey {
    /// The validated node-book target captured when the person submitted.
    #[must_use]
    pub const fn host(&self) -> &Name {
        &self.host
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
}

/// Explicit disposition prevents a stale response from closing a newer form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectReply {
    /// This response owns the current form's in-flight state.
    Current,
    /// Cache reconciliation may still use it, but the current draft must remain untouched.
    Stale,
}

/// State is pure and owned by one screen; transport work stays in link.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectActions {
    serial: u64,
    pending: Option<ProjectActionKey>,
}
impl ProjectActions {
    /// Capture the chosen host before a node call is submitted.
    ///
    /// # Errors
    /// A pending action or exhausted identifier space refuses duplicate dispatch.
    pub fn begin(&mut self, host: Name) -> Result<ProjectActionKey, ProjectActionError> {
        if self.pending.is_some() {
            return Err(ProjectActionError::Busy);
        }
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or(ProjectActionError::Exhausted)?;
        let key = ProjectActionKey {
            host,
            serial: self.serial,
        };
        self.pending = Some(key.clone());
        Ok(key)
    }
    /// Cancelling detaches the draft, without claiming the remote side effect was undone.
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    /// A queue refusal, transport failure and successful reply settle through the same key.
    #[must_use]
    pub fn settle(&mut self, key: &ProjectActionKey) -> ProjectReply {
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
        let key = state.begin(host("laptop")).unwrap();
        assert_eq!(state.begin(host("laptop")), Err(ProjectActionError::Busy));
        assert_eq!(state.pending(), Some(&key));
    }
    #[test]
    fn cancelled_completion_cannot_close_a_new_form_on_the_same_host() {
        let mut state = ProjectActions::default();
        let old = state.begin(host("laptop")).unwrap();
        state.cancel();
        let new = state.begin(host("laptop")).unwrap();
        assert_eq!(state.settle(&old), ProjectReply::Stale);
        assert_eq!(state.pending(), Some(&new));
        assert_eq!(state.settle(&new), ProjectReply::Current);
    }
    #[test]
    fn switching_hosts_never_settles_the_other_hosts_form() {
        let mut state = ProjectActions::default();
        let old = state.begin(host("laptop")).unwrap();
        state.cancel();
        let new = state.begin(host("build-server")).unwrap();
        assert_eq!(state.settle(&old), ProjectReply::Stale);
        assert_eq!(new.host().as_str(), "build-server");
        assert_eq!(state.pending(), Some(&new));
    }
    #[test]
    fn identifier_exhaustion_never_wraps() {
        let mut state = ProjectActions {
            serial: u64::MAX,
            pending: None,
        };
        assert_eq!(
            state.begin(host("laptop")),
            Err(ProjectActionError::Exhausted)
        );
        assert!(state.pending().is_none());
    }
}
