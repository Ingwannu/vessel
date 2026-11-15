//! One command-set read and confirmation retain the original shared admission.
//! This is view correlation only; the node still verifies scopes and digests.
use crate::connection::ConnectionContext;
use crate::workspace::command_changes::{Listed, compare_approved};
use crate::workspace::git::{Reading, RequestKey, RequestKeys};
use crate::workspace::problem::Problem;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{CommandsDigest, ProjectCommands};

/// A project's commands on one immutable authenticated admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Shared actor-produced identity, never a per-area epoch.
    pub context: ConnectionContext,
    /// Node-local project resource.
    pub project: ProjectId,
}
/// Exact complete snapshot that was shown before approving or withdrawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    /// Original producing connection and project.
    pub target: Target,
    /// Digest the person inspected.
    pub digest: CommandsDigest,
    snapshot: RequestKey,
}
/// Persists beyond detail recreation so old replies cannot reuse a read number.
#[derive(Debug, Default)]
pub struct Model {
    keys: RequestKeys,
    /// Currently displayed owner.
    pub target: Option<Target>,
    /// A loading/failed read cannot authorize a mutation.
    pub reading: Reading<ProjectCommands>,
    /// Pending original mutation; never replayed automatically.
    pub writing: Option<(RequestKey, CommandsDigest, bool)>,
    /// Safe last mutation failure.
    pub failure: Option<Problem>,
    snapshot: Option<RequestKey>,
    /// The lifetime-long request issuer cannot allocate another safe identity.
    pub exhausted: bool,
}
impl Model {
    /// Reopening invalidates old receipts while preserving the key issuer.
    pub fn select(&mut self, target: Option<Target>) {
        self.target = target;
        self.reading = Reading::Empty;
        self.writing = None;
        self.failure = None;
        self.snapshot = None;
    }
    /// Explicit reads may choose a new admission; mutations cannot retarget.
    pub fn begin_read(&mut self, target: Target) -> Option<RequestKey> {
        if self.target.as_ref() != Some(&target) {
            self.target = Some(target);
            self.reading = Reading::Empty;
            self.writing = None;
            self.failure = None;
            self.snapshot = None;
        }
        if self.reading.outstanding().is_some() || self.writing.is_some() {
            return None;
        }
        let key = self.key()?;
        self.failure = None;
        self.reading.start(key);
        Some(key)
    }
    /// Settle only the matching original read, even if an alias now exists again.
    pub fn accept_read(
        &mut self,
        target: &Target,
        key: RequestKey,
        result: Result<ProjectCommands, Problem>,
    ) -> bool {
        if self.target.as_ref() != Some(target) {
            return false;
        }
        let accepted = self.reading.accept(key, result);
        if accepted {
            self.snapshot = self.reading.ready().map(|_commands| key);
        }
        accepted
    }
    /// Capture the digest and receipt currently shown; no selection lookup later.
    #[must_use]
    pub fn ticket(&self) -> Option<Ticket> {
        if self.writing.is_some() {
            return None;
        }
        let listed = Listed::of(self.reading.ready()?)?;
        Some(Ticket {
            target: self.target.clone()?,
            digest: listed.digest.clone(),
            snapshot: self.snapshot?,
        })
    }
    /// Approval needs complete supported display. Withdrawal only removes a grant.
    pub fn begin_write(&mut self, ticket: &Ticket, approved: bool) -> Option<RequestKey> {
        if self.ticket().as_ref() != Some(ticket) {
            return None;
        }
        let listed = Listed::of(self.reading.ready()?)?;
        if listed.approved == approved
            || (approved && compare_approved(&listed, None).approval(&listed).is_err())
        {
            return None;
        }
        let key = self.key()?;
        self.writing = Some((key, ticket.digest.clone(), approved));
        self.failure = None;
        Some(key)
    }
    /// A timed-out grant remains uncertain and requires an explicit fresh read.
    pub fn accept_write(
        &mut self,
        target: &Target,
        key: RequestKey,
        digest: &CommandsDigest,
        approved: bool,
        result: Result<ProjectCommands, Problem>,
    ) -> bool {
        if self.target.as_ref() != Some(target)
            || !self.writing.as_ref().is_some_and(|pending| {
                pending.0 == key && &pending.1 == digest && pending.2 == approved
            })
        {
            return false;
        }
        self.writing = None;
        self.snapshot = None;
        match result {
            Ok(commands) => {
                self.reading = Reading::Ready(commands);
                self.snapshot = Some(key);
                self.failure = None;
            }
            Err(problem) => {
                self.reading = Reading::Empty;
                self.failure = Some(problem);
            }
        }
        true
    }
    fn key(&mut self) -> Option<RequestKey> {
        match self.keys.issue() {
            Ok(key) => Some(key),
            Err(_exhausted) => {
                self.exhausted = true;
                None
            }
        }
    }
}
#[cfg(test)]
#[path = "command_access/tests.rs"]
mod tests;
