//! Local admission/reply failures remain distinct from a node's wire refusal.
//! Reuse the existing correlated read lifecycle rather than fabricating an
//! machine error code or leaving a request loading when its route cannot be used.

use vessel_proto::workspace::{RefName, RelPath, WorktreeBase};
use vessel_proto::{NodeId, SessionId};

use crate::workspace::git::ReadState;
use crate::workspace::problem::Problem;

/// A correlated panel read includes typed local preflight/validation failures.
pub type PanelRead<T> = ReadState<T, ReadFailure>;

/// A recovery is chosen from the typed failure, never a diagnostic substring.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadFailure {
    /// Successful comparison admission found no registered or explicit base.
    #[error("there is no comparison base")]
    NoBase,
    /// The protected reader refused the complete output budget.
    #[error("the comparison exceeds the preview limit")]
    TooLarge,
    /// No authoritative root has been provided for this workspace.
    #[error("workspace repository information is unavailable")]
    WorkspaceUnavailable,
    /// Lifetime request identities must never wrap or be reused.
    #[error("panel request identities exhausted")]
    KeysExhausted,
    /// The original physical connection closed; no alias fallback is attempted.
    #[error("the selected connection closed")]
    DestinationClosed,
    /// App-side bounded admission is full, before a request is polled.
    #[error("too many panel reads are active")]
    Capacity,
    /// The bounded call deadline elapsed, without proving transport closure.
    #[error("the panel read did not finish in time")]
    TimedOut,
    /// Preserve the node/transport problem and its diagnostic intact.
    #[error("the machine could not answer the git read")]
    Call(Problem),
    /// The selected route or its credential/admission epoch was retired.
    #[error("the selected connection changed")]
    DestinationChanged,
    /// The exact selected authenticated client answered for another node.
    #[error("expected node {expected}, received {received}")]
    UnexpectedNode {
        /// Expected identity from the selected machine, never an alias search.
        expected: NodeId,
        /// Identity read from the actual authenticated client.
        received: NodeId,
    },
    /// A body cannot be installed under the path that happened to request it.
    #[error("expected diff for {requested}, received {received}")]
    UnexpectedFile {
        /// The file the person opened in this comparison.
        requested: RelPath,
        /// The path carried by the returned body.
        received: RelPath,
    },
    /// A broken peer returned another session's metadata.
    #[error("expected session {requested}, received {received}")]
    UnexpectedSession {
        /// Session explicitly named by the pane key.
        requested: SessionId,
        /// Session carried by the reply, never adopted as authority.
        received: SessionId,
    },
    /// Retain both identities without declaring either branch correct.
    #[error("the stored and requested comparison bases differ")]
    BaseMismatch {
        /// Recorded worktree ref, before the project-default fallback.
        stored: RefName,
        /// Different explicit base requested by the person.
        requested: WorktreeBase,
    },
    /// An older/pending node cannot provide committed-only semantics.
    #[error("the selected comparison is unavailable on this connection")]
    ComparisonUnavailable,
}

impl From<Problem> for ReadFailure {
    fn from(problem: Problem) -> Self {
        Self::Call(problem)
    }
}
