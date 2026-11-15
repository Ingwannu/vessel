//! Runtime states are snapshots; only the live actor can admit an invocation.

use std::{fmt, sync::Arc, time::Duration};

use tokio::sync::{OwnedSemaphorePermit, mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use vessel_proto::plugins::{
    ClientDocument, HelperCall, HelperFailure, HelperPayload, HookSubscriptions, PluginAddress,
};

/// Same outer callback budget as Paseo runtime.ts:41.
pub const REQUEST_DEADLINE: Duration = Duration::from_secs(30);
/// Grace for cooperative dispose, then another grace after termination.
pub const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);
/// Queued and active calls share this byte admission, retained until settled.
pub const MAX_ADMITTED_BYTES: usize = 8 * 1024 * 1024;
/// Small requests also have a count bound independent of payload size.
pub const MAX_PENDING_CALLS: usize = 16;
/// Dedicated pipe workers never accumulate unbounded frame queues.
pub(crate) const PIPE_QUEUE: usize = 2;
/// Accounts for typed names/envelope bookkeeping in weighted admission.
pub(crate) const CALL_OVERHEAD_BYTES: usize = 1024;

/// A complete catalog snapshot, published after validation or withdrawn on exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeState {
    /// No helper callback or native action may run before ready.
    Starting,
    /// Core-assigned provenance; no diagnostic/private broker data included.
    Running {
        /// Exact public semantic generation, rotated on declaration changes.
        address: PluginAddress,
        /// One immutable validated graph.
        client: Arc<ClientDocument>,
        /// Complete current lifecycle subscription set.
        hooks: HookSubscriptions,
    },
    /// All live contribution/action authority was withdrawn.
    Stopped,
}

/// Errors safe to project into a scoped, localized client failure. Private
/// helper diagnostics and pipe/parser/OS details stay with the supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CallError {
    /// Node/installation/generation changed before actor admission.
    #[error("plugin installation changed")]
    Stale,
    /// The child has not published its first complete declarations.
    #[error("plugin helper is not ready")]
    NotReady,
    /// This exact generation does not advertise the requested method/hook.
    #[error("plugin callback is not registered")]
    Unregistered,
    /// No request/byte/pipe slot was admitted; nothing was dispatched.
    #[error("plugin helper request capacity is full")]
    Busy,
    /// The caller dropped/cancelled its operation.
    #[error("plugin helper request was cancelled")]
    Cancelled,
    /// A finite callback/initialization budget elapsed.
    #[error("plugin helper request timed out")]
    Timeout,
    /// Process, runtime or request receiver ended.
    #[error("plugin helper is unavailable")]
    Closed,
    /// Safe helper classification, never its arbitrary diagnostic text.
    #[error("plugin callback failed ({0:?})")]
    Failed(HelperFailure),
}

pub(crate) struct Request {
    pub(crate) address: PluginAddress,
    pub(crate) call: HelperCall,
    pub(crate) reply: oneshot::Sender<Result<HelperPayload, CallError>>,
    pub(crate) deadline: tokio::time::Instant,
    pub(crate) cancel: CancellationToken,
    pub(crate) bytes: OwnedSemaphorePermit,
    pub(crate) count: OwnedSemaphorePermit,
}

// Opaque data/capabilities must not be printed accidentally by actor diagnostics.
impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Request(<private callback>)")
    }
}

pub(crate) type Requests = mpsc::Receiver<Request>;
pub(crate) type StateSender = watch::Sender<RuntimeState>;
