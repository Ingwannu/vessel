//! Typed RPC: the method table, the frames, and the generated dispatcher.
//!
//! [Decision Log]
//! - Purpose: every request must name its response type, and the node must
//!   be unable to compile while any method is unhandled.
//! - Alternatives: REST routes with per-route JSON (no shared method table,
//!   unhandled routes are runtime 404s); Zed's `TypeId`-keyed handler map
//!   with `downcast().unwrap()` (dynamic typing at the core); Paseo's
//!   request id inside each payload with one predicate scan per pending call.
//! - Choice: one table (`method_table!` in table.rs, expanded through
//!   `rpc_methods!` below) generates `Method`, the `Call`
//!   and `Reply` enums, the `Request` impls binding each request to its
//!   response, a `Handler` trait with one method per call, and `dispatch`,
//!   an exhaustive `match`. Correlation ids live in the frame, not the
//!   payload.
//! - Cost: adding a method means editing this table (deliberate: it is the
//!   one place the protocol is defined) and every `Handler` implementation.
//!
//! [Decision Log: the exported table] (Quality B, milestone 12)
//! - Purpose: every new method also had to be added by hand to the node's
//!   guard rows, the envelope's forwarding arms and the client's unary
//!   lists (about 4 lines in 4 files per method), and a method whose number
//!   was not assigned yet could not have a row at all.
//! - Alternatives: keep the hand lists and raise the files' size budget
//!   with every call; a second "pending" table beside this one; a dummy
//!   version for unassigned methods. The first grows four files per call,
//!   the other two give the protocol two sources of truth or a number that
//!   means nothing.
//! - Choice: the table is the exported `method_table!` macro, which hands
//!   its rows to a callback; this crate, the node and the client generate
//!   their per-method code from the same rows. Two columns were added: the
//!   protocol column accepts `(pending FEATURE)` (and, since 2026-11-15,
//!   `(staged FEATURE)` for values that land before their runtime; see
//!   methods.rs), and the shape column (`MethodShape`) says what the hand
//!   lists used to say.
//! - Cost: types in rows are written from the crate root (`$crate::`), and
//!   `Method::since()` / `Call::since()` return an `Option`, so tests that
//!   compare them wrap the expected version in `Some`.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::device::Scope;
use crate::error::WireError;
use crate::node::ProtocolVersion;

mod contract;
mod frames;
mod older;
mod requests;
#[cfg(test)]
pub(crate) mod samples;
mod shape;
mod since;
mod table;

pub use contract::{Request, UnexpectedReply};
pub use frames::{ClientFrame, ClientInfo, Hello, ServerFrame, SubscriptionEnd, Welcome};
pub use requests::{
    Ack, CreateSession, EventPage, GetNode, GetSession, KillSession, ListHarnesses, ListSessions,
    ReadEvents, RenameSession, SendInput, SetLabels, StopNode, StopWhen, Subscribe, Subscribed,
    Unsubscribe,
};
pub use shape::MethodShape;

#[macro_use]
mod methods;

// The table itself is in `table.rs` (`method_table!`, exported at the crate
// root); this invocation generates this module's enums and `Handler`.
crate::method_table!(rpc_methods);

/// Result of a call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "one Outcome per call, moved once to the writer and never stored in bulk; \
              boxing the reply would add an allocation to every call to save a copy"
)]
pub enum Outcome {
    /// Success.
    Ok(Reply),
    /// Failure.
    Err(WireError),
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod conversation_tests;

#[cfg(test)]
mod staged_tests;
