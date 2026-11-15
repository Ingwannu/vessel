//! Schedule RPCs with bounded work and source-preserving replies. Fleet and
//! conversation traffic never wait for a scheduled agent's run-now completion.
//!
//! [Decision Log]
//! - Purpose: one answer for each correlated list or explicitly chosen write.
//! - Constraints: reconnect/navigation can make a reply obsolete; losing a
//!   write's reply does not mean the node failed to perform it.
//! - Alternatives: inline actor awaits or unbounded detached calls.
//! - Choice: the view's shared named permit bound, retained through event
//!   delivery, no write retry, and full errors until the presentation edge.
//! - Why and cost: slow calls cannot stall fleet traffic or grow without a
//!   limit. Saturation returns an explicit Busy answer instead of a dead click.
pub use vessel_app_view::schedules::page::Request;
use vessel_app_view::schedules::{
    load::Failure,
    page::{Ask, WriteValue},
};
use vessel_proto::{
    automation::ScheduleInfo,
    workspace::{ProjectInfo, WorktreeInfo},
};

/// Authoritative list plus independently fallible target facts from its node.
#[derive(Debug)]
pub struct ListValue {
    /// Exactly the node's schedule list; no locally synthesized states.
    pub schedules: Vec<ScheduleInfo>,
    /// Read completion time, used only for refreshing the cache.
    pub read_at: chrono::DateTime<chrono::Utc>,
    /// No dependency on visiting a separate workspace screen first.
    pub projects: Result<Vec<ProjectInfo>, ScheduleError>,
    /// Absence/error never proves a worktree target was deleted.
    pub worktrees: Result<Vec<WorktreeInfo>, ScheduleError>,
}
/// Reply payload; its request remains the correlation authority.
#[derive(Debug)]
pub enum Value {
    /// Node schedule and target facts.
    List(Box<ListValue>),
    /// The node acknowledged an explicitly requested mutation.
    Written(WriteValue),
}
/// One area notice, distinct from any RPC ticket or optimistic state change.
#[derive(Debug)]
pub enum Message {
    /// The actor established a fresh connection for this exact book key.
    Connected(vessel_app_view::schedules::load::MachineKey),
    /// A correlated response to a previously issued operation.
    Replied(Answer),
}

/// Complete response, retaining every transport error until the app consumes it.
#[derive(Debug)]
pub struct Answer {
    /// Echoed exactly, so a stale reply cannot settle a newer operation.
    pub request: Request,
    /// No diagnostic Display string is used as visible copy.
    pub result: Result<Value, ScheduleError>,
}
/// Transport/source errors, independent of their localized presentation.
#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    /// The original typed node or transport refusal.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("schedule call: {0}")]
    Call(#[from] vessel_client::CallError),
    /// No call was dispatched because its machine has no connection.
    #[error("schedule machine is not connected")]
    Offline,
    /// No call was dispatched because the local pool is full.
    #[error("schedule request pool is full")]
    Busy,
    /// Browser builds have no node transport; never return invented schedules.
    #[error("schedule node transport is unavailable in this browser build")]
    NoTransport,
}
impl ScheduleError {
    /// Classify once where the app prepares its words. Only disconnected or
    /// mismatched write replies are uncertain; unsent requests remain distinct.
    #[must_use]
    pub fn failure(&self, ask: Ask) -> Failure {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(error) => match error {
                vessel_client::CallError::Remote(wire) => Failure::Node(wire.clone()),
                vessel_client::CallError::NotInProtocol { .. } => Failure::Older,
                vessel_client::CallError::RequestIdsExhausted => Failure::Exhausted,
                vessel_client::CallError::Disconnected => match ask {
                    Ask::List => Failure::Offline,
                    Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                        Failure::Unconfirmed
                    }
                },
                vessel_client::CallError::UnexpectedReply(_) => match ask {
                    Ask::List => Failure::Unexpected,
                    Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                        Failure::Unconfirmed
                    }
                },
            },
            Self::Offline => Failure::Offline,
            Self::Busy => Failure::Busy,
            Self::NoTransport => match ask {
                Ask::List | Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                    Failure::NoTransport
                }
            },
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
