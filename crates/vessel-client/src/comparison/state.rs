//! Passive reports cannot recreate a continuation or retry a mutation.
use super::Selection;
use crate::CallError;

#[derive(Debug, thiserror::Error)]
pub(super) enum Cause {
    #[error(transparent)]
    Rpc(#[from] CallError),
    #[error(transparent)]
    Count(#[from] std::num::TryFromIntError),
}
use serde::Serialize;
use std::sync::Arc;
use vessel_proto::workspace::BranchName;
use vessel_proto::{AttachmentId, ErrorCode, NodeId, SessionId, UploadId, WorktreeId};

/// Position of one selection in the validated, at-most-eight plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SelectionIndex(pub(super) usize);
impl SelectionIndex {
    /// Zero-based input position.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
/// Where an admitted operation was interrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Shared read-only checks.
    Preflight,
    /// Make a new branch/worktree.
    WorktreeCreate,
    /// Observe existing setup.
    Setup,
    /// Create this target's one session.
    SessionCreate,
    /// Observe this session's ready state.
    Ready,
    /// Common-input capability checks.
    Validate,
    /// Begin one destination upload.
    UploadBegin,
    /// Read stored source bytes.
    AttachmentRead,
    /// Write a destination piece.
    UploadWrite,
    /// Finish a destination upload.
    UploadFinish,
    /// Start the initial turn, never queue or steer.
    Prompt,
}
/// Safe reason class, separate from the retained concrete RPC source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// A typed node/RPC refusal.
    Call,
    /// No reply within the operation's deadline.
    Deadline,
    /// The owner explicitly cancelled or dropped.
    Cancelled,
    /// Another selection stopped shared preflight.
    PreflightStopped,
    /// Underlying comparison calls are unavailable on this peer.
    Protocol,
    /// Worktree creation was definitively refused.
    Worktree,
    /// Project is not ready Git, or base has no commit.
    Project,
    /// No structured handshake is available for a selected provider.
    Provider,
    /// The agent ended before becoming ready.
    AgentEnded,
    /// Setup failed or has an unknown state.
    Setup,
    /// Input exceeds a target's abilities or limits.
    Input,
    /// Source is not an eligible member of this job.
    Source,
    /// A source size, chunk, offset or returned reference mismatched.
    Copy,
    /// Copy exceeded its finite total/progress budget.
    CopyBudget,
}
/// Typed failure retains its actual source without serializing it into a report.
#[derive(Debug, Clone, thiserror::Error)]
#[error("comparison failed during {phase:?}: {kind:?}")]
pub struct Failure {
    /// Stable operation phase.
    pub phase: Phase,
    /// Stable reason.
    pub kind: FailureKind,
    /// A node's error class, if any.
    pub code: Option<ErrorCode>,
    /// Definitive worktree refusal, without flattening it to text.
    pub refusal: Option<vessel_proto::workspace::WorktreeRefusal>,
    #[source]
    pub(super) source: Option<Arc<Cause>>,
}
impl Failure {
    /// The node's original typed error, including its explanation. Non-node
    /// transport/deadline failures have their own kind and return None.
    #[must_use]
    pub fn remote_error(&self) -> Option<&vessel_proto::WireError> {
        if let Some(Cause::Rpc(CallError::Remote(error))) = self.source.as_deref() {
            Some(error)
        } else {
            None
        }
    }
    pub(super) fn count(phase: Phase, source: std::num::TryFromIntError) -> Self {
        Self {
            source: Some(Arc::new(Cause::Count(source))),
            ..Self::new(phase, FailureKind::Copy)
        }
    }
    pub(super) const fn new(phase: Phase, kind: FailureKind) -> Self {
        Self {
            phase,
            kind,
            code: None,
            refusal: None,
            source: None,
        }
    }
}
impl Serialize for Failure {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Report<'a> {
            phase: Phase,
            kind: FailureKind,
            code: Option<ErrorCode>,
            refusal: &'a Option<vessel_proto::workspace::WorktreeRefusal>,
            #[serde(skip_serializing_if = "Option::is_none")]
            remote: Option<&'a vessel_proto::WireError>,
        }
        Report {
            phase: self.phase,
            kind: self.kind,
            code: self.code,
            refusal: &self.refusal,
            remote: self.remote_error(),
        }
        .serialize(serializer)
    }
}
/// Exact copy trail survives a lost begin/write/finish reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyProgress {
    /// Original immutable source session.
    pub source: SessionId,
    /// Its stored file.
    pub attachment: AttachmentId,
    /// Position in common input.
    pub attachment_index: usize,
    /// Unknown if begin's reply was lost.
    pub upload: Option<UploadId>,
    /// Destination id actually returned by finish, including rejected metadata.
    /// None means no finish reply was received for this file.
    pub finished: Option<AttachmentId>,
    /// Bytes acknowledged by the destination.
    pub acknowledged: u64,
    /// Offset of the admitted write/read.
    pub attempted_offset: u64,
    /// Length of its admitted write, zero for a read/begin/finish.
    pub attempted_bytes: u64,
}
/// Recovery identities, not instructions to replay work.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceTrail {
    /// Always known before creation is admitted.
    pub branch: BranchName,
    /// Known after worktree creation replies.
    pub worktree: Option<WorktreeId>,
    /// Known after session creation replies.
    pub session: Option<SessionId>,
    /// Finished destination files already acknowledged before a later failure.
    pub attachments: Vec<vessel_proto::AttachmentRef>,
    /// Latest admitted/acknowledged copy step.
    pub copy: Option<CopyProgress>,
}
/// One selection's current state. Uncertainty is terminal in this job.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", content = "reason", rename_all = "snake_case")]
pub enum TargetStatus {
    /// No mutation yet.
    Planned,
    /// Exact phase/resources identify the admitted mutation/read.
    InFlight,
    /// Setup is still pending, with the same worktree.
    WaitingSetup,
    /// Readiness is still pending, with the same session.
    WaitingReady,
    /// No worktree made; ordinary explicit approval is needed.
    NeedsApproval(Arc<vessel_proto::workspace::ProjectCommands>),
    /// Can take its one initial prompt.
    Ready,
    /// The initial prompt was acknowledged once.
    Started,
    /// Terminal known failure, retaining partial resources.
    Failed(Failure),
    /// An admitted mutation may have taken effect. No automatic retry.
    Uncertain(Failure),
    /// Cancelled without an admitted unresolved mutation.
    Cancelled,
}
/// Passive state for one input position.
#[derive(Debug, Clone, Serialize)]
pub struct TargetReport {
    /// Stable position.
    pub index: SelectionIndex,
    /// Submitted provider/model metadata.
    pub selection: Arc<Selection>,
    /// Current phase.
    pub phase: Phase,
    /// State, never an executable continuation.
    pub status: TargetStatus,
    /// Known recovery identities.
    pub resources: ResourceTrail,
}
/// Whether the owner can still command the job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Owner is alive; no new work except through its commands.
    Active,
    /// Owner cancellation settled; no more calls.
    Cancelled,
}
/// Latest bounded report; cloning one grants no mutation capability.
///
/// ```compile_fail
/// use vessel_client::comparison::Snapshot;
/// let restored = serde_json::from_str::<Snapshot>("{}");
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    /// Selected node, also retained in every RPC route.
    pub destination: NodeId,
    /// Owner lifetime.
    pub status: JobStatus,
    /// Input-order outcomes, at most eight.
    pub targets: Vec<TargetReport>,
    /// Shared validation failed before binding, if any.
    pub input_rejected: Option<Failure>,
    /// Common input is immutable from this point.
    pub input_bound: bool,
}
