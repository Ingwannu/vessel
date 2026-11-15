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
mod clock;
pub use clock::ViewerClockError;
pub use vessel_app_view::schedules::page::Request as RowRequest;
mod request;
pub use request::{FormAnswer, HistoryAnswer, Request, SessionAnswer};
use vessel_app_view::schedules::page::WriteValue;
use vessel_proto::{
    automation::ScheduleInfo,
    workspace::{ProjectInfo, WorktreeInfo},
};

/// Authoritative list plus independently fallible target facts from its node.
#[derive(Debug)]
pub struct ListValue {
    /// Exactly the node's schedule list; no locally synthesized states.
    pub schedules: Vec<ScheduleInfo>,
    /// Viewer platform calendar, never the remote machine's zone.
    pub clock: Result<vessel_app_view::schedules::clock::Clock, ViewerClockError>,
    /// Effective grant from device.self; an unavailable report is never a denial.
    pub scopes: Result<Vec<vessel_proto::Scope>, ScheduleError>,
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
    /// A typed editor read/save reply with its original form request.
    FormReplied(FormAnswer),
    /// Read-only history with its original schedule/admission/ticket.
    HistoryReplied(HistoryAnswer),
    /// Historical session metadata only, under the original physical admission.
    SessionReplied(SessionAnswer),
}

/// Complete response, retaining every transport error until the app consumes it.
#[derive(Debug)]
pub struct Answer {
    /// Echoed exactly, so a stale reply cannot settle a newer operation.
    pub request: RowRequest,
    /// No diagnostic Display string is used as visible copy.
    pub result: Result<Value, ScheduleError>,
}
mod error;
pub use error::{CallFailure, ScheduleError};
#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
mod form;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
