//! Retain the original typed refusal privately, without exposing a peer's
//! message through automatic Debug/Display/source-chain formatting.
use vessel_app_view::connection::Refusal;
use vessel_app_view::schedules::{load::Failure, page::Ask};

/// Original typed transport error; its presentation is deliberately redacted.
#[derive(thiserror::Error)]
#[error("the selected machine refused the schedule request")]
pub struct CallFailure(#[cfg(not(target_arch = "wasm32"))] vessel_client::CallError);
impl std::fmt::Debug for CallFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CallFailure(<redacted>)")
    }
}
/// Errors retain their stage: unsent admission refusal is not a lost write reply.
#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    /// Original typed failure kept behind a non-disclosing source wrapper.
    #[error("schedule call: {0}")]
    Call(#[source] CallFailure),
    /// No RPC was polled because its selected physical admission no longer exists.
    #[error("schedule admission: {0}")]
    NotSelected(#[source] Refusal),
    /// The original admission retired during an operation; a write may have run.
    #[error("schedule connection retired: {0}")]
    Retired(#[source] Refusal),
    /// The bounded operation did not confirm a result before its deadline.
    #[error("schedule request exceeded its deadline")]
    Deadline,
    /// No connected transport was selected; legacy local queue refusal.
    #[error("schedule machine is not connected")]
    Offline,
    /// No request was dispatched because the local pool is full.
    #[error("schedule request pool is full")]
    Busy,
    /// Browser builds have no transport; never invent a schedule result.
    #[error("schedule transport is unavailable in this browser build")]
    NoTransport,
}
#[cfg(not(target_arch = "wasm32"))]
impl From<vessel_client::CallError> for ScheduleError {
    fn from(error: vessel_client::CallError) -> Self {
        Self::Call(CallFailure(error))
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl From<crate::connections::RequestFailure> for ScheduleError {
    fn from(error: crate::connections::RequestFailure) -> Self {
        match error {
            crate::connections::RequestFailure::Refused(why) => Self::Retired(why),
            crate::connections::RequestFailure::Call(error) => Self::from(error),
        }
    }
}
fn lost(ask: Ask) -> Failure {
    match ask {
        Ask::List => Failure::Offline,
        Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => Failure::Unconfirmed,
    }
}
impl ScheduleError {
    /// Classify once at presentation; diagnostic prose never becomes UI copy.
    #[must_use]
    pub fn failure(&self, ask: Ask) -> Failure {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(CallFailure(error)) => match error {
                vessel_client::CallError::Remote(wire) => Failure::Node(wire.clone()),
                vessel_client::CallError::NotInProtocol { .. } => Failure::Older,
                vessel_client::CallError::UnassignedProtocol { .. } => Failure::NotEnabled,
                vessel_client::CallError::RequestIdsExhausted => Failure::Exhausted,
                vessel_client::CallError::Disconnected => lost(ask),
                vessel_client::CallError::UnexpectedReply(_) => match ask {
                    Ask::List => Failure::Unexpected,
                    Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                        Failure::Unconfirmed
                    }
                },
            },
            #[cfg(target_arch = "wasm32")]
            Self::Call(_) => Failure::NoTransport,
            Self::Retired(why) => match ask {
                Ask::List => connection(*why),
                Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                    Failure::Unconfirmed
                }
            },
            Self::Deadline => match ask {
                Ask::List => Failure::TimedOut,
                Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => {
                    Failure::Unconfirmed
                }
            },
            Self::NotSelected(why) => connection(*why),
            Self::Offline => Failure::Offline,
            Self::Busy => Failure::Busy,
            Self::NoTransport => Failure::NoTransport,
        }
    }
}

fn connection(why: Refusal) -> Failure {
    match why {
        Refusal::Changed => Failure::ConnectionChanged,
        Refusal::Closed => Failure::Offline,
    }
}
