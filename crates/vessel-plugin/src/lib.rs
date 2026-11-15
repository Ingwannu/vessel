//! Trusted node-account plugin helper supervision (ADR 0079).
//!
//! This crate owns framing, finite admission/cancellation, atomic declarations,
//! private log tails and one process-group lifetime. It knows no client token,
//! node method, installation source, manifest filesystem, vendor provider or
//! workspace ownership. The node authorizes before calling it and revalidates
//! returned business values. A helper is unsandboxed trusted account code.
//!
//! [Decision Log]
//! - Purpose: keep slow/failing plugin callbacks out of the node event loop
//!   without creating a second authenticated RPC or process cleanup mechanism.
//! - Reuse: proto owns typed private envelopes; proc owns launch/group/job kill.
//! - Choice: bounded pipe workers and one pending-request actor; count and bytes
//!   stay admitted until settlement or process retirement, including cancellation.
//! - Tradeoff: a stuck callback/control queue retires its helper generation,
//!   sacrificing that installation's availability to preserve finite ownership.

mod actor_core;
mod handle;
mod launch;
mod log_tail;
mod runtime_error;
mod runtime_types;
mod shared_logs;
mod shutdown;
mod supervisor;
pub mod wire;
mod workers;

pub use handle::RuntimeHandle;
pub use launch::{
    LaunchRefusal, MAX_HELPER_ARGUMENT_BYTES, MAX_HELPER_ARGUMENTS, MAX_HELPER_ENVIRONMENT_BYTES,
    PreparedCommand,
};
pub use log_tail::{LogExhausted, LogTail};
pub use runtime_error::{ProtocolViolation, RuntimeError, TaskFailure};
pub use runtime_types::{
    CallError, MAX_ADMITTED_BYTES, MAX_PENDING_CALLS, REQUEST_DEADLINE, RuntimeState,
    SHUTDOWN_GRACE,
};
pub use shared_logs::SharedLogs;
pub use supervisor::Supervisor;
