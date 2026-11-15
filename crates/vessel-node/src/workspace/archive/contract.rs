//! Minimal workflow ports project canonical identity without duplicating registry DTOs.
use std::{error::Error, future::Future};
use vessel_proto::{SessionId, TerminalId, WorkspaceId, workspace::RemoveWorktreeOutcome};

/// Exact resource identity from the registry membership snapshot, never from cwd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Member {
    Session(SessionId),
    Terminal(TerminalId),
}
/// The durable claim is opaque: only the registry adapter can validate its revision.
pub(crate) enum Claim<C> {
    Acquired(C),
    AlreadyArchived,
}
/// Preserve the existing worktree safety result instead of inventing deletion proof.
#[derive(Debug)]
pub(crate) enum Backing {
    LocalPreserved,
    SharedPreserved,
    Managed(RemoveWorktreeOutcome),
}
/// The failed stage remains distinct from an unknown overall timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    Claim,
    Setup,
    Starts,
    Members,
    Drain,
    Backing,
    Commit,
}
/// Partial outcomes retain the underlying error for the service boundary.
#[derive(Debug)]
pub(crate) enum Failure<E> {
    At {
        stage: Stage,
        member: Option<Member>,
        source: E,
    },
    Deadline,
}
/// An incomplete drain never reports a successful archive or authorizes restore.
#[derive(Debug)]
pub(crate) enum ArchiveOutcome<E> {
    AlreadyArchived,
    Archived { backing: Backing },
    Incomplete(Vec<Failure<E>>),
}
/// The registry/resource adapter owns mutation authority and physical postconditions.
/// Calls continuing after cancellation must retain the durable claim until reconciled.
pub(crate) trait ArchiveBackend: Sync {
    type Claim: Send + Sync;
    type Error: Error + Send + Sync + 'static;
    /// Persist an exclusive admission fence before any setup/resource is stopped.
    /// A concurrent caller cannot reuse an in-progress claim. Recovery of an
    /// abandoned claim requires the adapter's authoritative reconciliation.
    fn claim(
        &self,
        workspace: WorkspaceId,
    ) -> impl Future<Output = Result<Claim<Self::Claim>, Self::Error>> + Send;
    /// Prove setup stopped; a timeout/refusal cannot be treated as completion.
    fn stop_setup(
        &self,
        claim: &Self::Claim,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    /// Wait for prior start leases and reconcile uncertainty before reading members.
    fn settle_starts(
        &self,
        claim: &Self::Claim,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    /// Read exact admitted members after earlier actors have registered.
    fn members(
        &self,
        claim: &Self::Claim,
    ) -> impl Future<Output = Result<Vec<Member>, Self::Error>> + Send;
    /// Archive/close idempotently, proving the target ended before answering success.
    fn stop_member(
        &self,
        claim: &Self::Claim,
        member: Member,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    /// Recheck references, trust and ownership; no unowned/shared path may be removed.
    /// The adapter also admits the resolved physical root through workspace
    /// exclusion. Its guard stays with actual I/O until cleanup/postconditions
    /// settle, even if the waiting workflow times out or is cancelled.
    fn cleanup_backing(
        &self,
        claim: &Self::Claim,
    ) -> impl Future<Output = Result<Backing, Self::Error>> + Send;
    /// Fresh CAS to Archived only after every required physical postcondition settled.
    fn finish(&self, claim: &Self::Claim) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
