//! Private node/helper pipe, not a second node RPC transport (ADR 0079).
//!
//! The initial executable slice supports lifecycle and native invocation only.
//! Unknown registration families fail decoding; providers, tools and usage must
//! not be advertised until their owning adapters are connected. This version
//! is the local helper format, independent of the pending `PLUGINS` feature.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use super::{
    ClientDocument, HelperPayload, HookName, HookSubscriptions, LogText, PluginAddress, RpcName,
};

/// Maximum complete JSON body, checked before allocating or decoding a frame.
pub const MAX_HELPER_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Version of the local framed format. Unsupported versions cannot become a
/// successfully initialized helper just because a number is positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum HelperProtocol {
    /// Bounded length-prefixed JSON, native declarations, explicit cancellation.
    V1,
}

/// Refused before catalog publication or executing a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unsupported plugin helper protocol")]
pub struct UnsupportedHelperProtocol;

impl TryFrom<u16> for HelperProtocol {
    type Error = UnsupportedHelperProtocol;
    fn try_from(version: u16) -> Result<Self, Self::Error> {
        match version {
            1 => Ok(Self::V1),
            _ => Err(UnsupportedHelperProtocol),
        }
    }
}

impl From<HelperProtocol> for u16 {
    fn from(version: HelperProtocol) -> Self {
        match version {
            HelperProtocol::V1 => 1,
        }
    }
}

/// Strictly increasing within one pipe lifetime, assigned only by its node.
/// A late retired id is ignored; an id never issued is a protocol violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvocationId(NonZeroU64);

impl InvocationId {
    /// Zero is reserved, so a missing/default correlation cannot acknowledge work.
    #[must_use]
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    /// Compare with the actor's monotonic allocation high-water mark.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// An operation on the private helper. It conveys no client authority. Hooks
/// receive typed core input serialized by their owner, and returned rewrites
/// must still pass that owner's locked-field and authorization checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HelperCall {
    /// Invoke one method from this generation's accepted declaration graph.
    Invoke {
        /// Validated registration name, never interpreted as a node method.
        method: RpcName,
        /// Opaque to the generic actor.
        input: HelperPayload,
    },
    /// Call one currently subscribed modern lifecycle hook.
    Hook {
        /// Core decides whether its caller waits using `HookName::wait`.
        name: HookName,
        /// Business types stay with the lifecycle owner.
        input: HelperPayload,
    },
}

/// Messages only the supervising node writes to the child's stdin. An adapter
/// loads its entry from node-prepared argv/cwd, not from a client-provided URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NodeToHelper {
    /// First message; no work is admitted before the matching ready response.
    Initialize {
        /// The local format the node actually speaks.
        protocol: HelperProtocol,
        /// Node-assigned provenance; the child cannot choose its identity.
        installation: PluginAddress,
    },
    /// A bounded admitted call.
    Request {
        /// Correlation assigned at actor admission.
        request: InvocationId,
        /// Closed operation family.
        call: HelperCall,
    },
    /// Cooperative cancellation. The host also drops the reply capability and
    /// fences subsequent responses; cancellation is not an acknowledgement.
    Cancel {
        /// Previously admitted call, never a future id.
        request: InvocationId,
    },
    /// Dispose plugin callbacks and owned resources before group termination.
    Shutdown {},
}

/// Safe classification separate from arbitrary private diagnostic text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperFailure {
    /// The plugin explicitly refused a valid call.
    Refused,
    /// A callback failed; do not present its arbitrary text as a scoped error.
    Callback,
    /// The cooperative child finished cancelling the call.
    Cancelled,
}

/// Child stdout is exclusively this protocol; stderr is bounded private logs.
/// Entire documents are validated before replacing the visible catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HelperToNode {
    /// Complete initial declaration set. Unknown unsupported server families
    /// cannot be silently omitted and mistaken for successful registration.
    Ready {
        /// Must match the initialized local version.
        protocol: HelperProtocol,
        /// Validated graph; includes declared invocation names.
        client: ClientDocument,
        /// Initial lifecycle subscriptions.
        hooks: HookSubscriptions,
    },
    /// Complete result for a live request.
    Result {
        /// The node's original correlation.
        request: InvocationId,
        /// Owning adapter validates business output before using it.
        output: HelperPayload,
    },
    /// Settled failure; management may inspect the private diagnostic.
    Error {
        /// The node's original correlation.
        request: InvocationId,
        /// Safe error classification.
        kind: HelperFailure,
        /// Never copied into a scoped client error or ambient trace.
        diagnostic: LogText,
    },
    /// Withdraw all live capabilities and stop this process generation.
    Fatal {
        /// Private management-only context.
        diagnostic: LogText,
    },
    /// Whole validated client graph, never a partly applied patch. The host
    /// rotates public generation before exposing semantic changes.
    ClientChanged {
        /// Complete replacement.
        client: ClientDocument,
    },
    /// Complete subscription replacement, checked for duplicates.
    HooksChanged {
        /// Replaces subscriptions atomically on the actor.
        hooks: HookSubscriptions,
    },
}
