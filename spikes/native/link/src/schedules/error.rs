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
    /// Local preference persistence refused before any schedule RPC was polled.
    #[error("the app could not save schedule choices")]
    Preferences,
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
#[derive(Clone, Copy)]
enum Effect {
    Read,
    Write,
}
impl Effect {
    fn of(ask: Ask) -> Self {
        match ask {
            Ask::List => Self::Read,
            Ask::Pause(_) | Ask::Resume(_) | Ask::Run(_) | Ask::Delete(_) => Self::Write,
        }
    }
    fn uncertain(self, read: Failure) -> Failure {
        match self {
            Self::Read => read,
            Self::Write => Failure::Unconfirmed,
        }
    }
}
impl ScheduleError {
    /// Read-only history never becomes an uncertain write on transport loss.
    #[must_use]
    pub fn read_failure(&self) -> Failure {
        self.for_effect(Effect::Read)
    }
    /// Existing row mapping; diagnostic prose never becomes UI copy.
    #[must_use]
    pub fn failure(&self, ask: Ask) -> Failure {
        self.for_effect(Effect::of(ask))
    }
    /// Save is a write even though create/update are not row menu actions.
    #[must_use]
    pub fn editor_failure(
        &self,
        operation: &vessel_app_view::schedules::editor::io::Operation,
    ) -> Failure {
        use vessel_app_view::schedules::editor::io::Operation;
        self.for_effect(match operation {
            Operation::Save(_) => Effect::Write,
            Operation::Targets | Operation::Options(_) | Operation::Preview(_) => Effect::Read,
        })
    }
    fn for_effect(&self, effect: Effect) -> Failure {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(CallFailure(error)) => match error {
                vessel_client::CallError::Remote(wire) => Failure::Node(wire.clone()),
                vessel_client::CallError::NotInProtocol { .. } => Failure::Older,
                vessel_client::CallError::UnassignedProtocol { .. } => Failure::NotEnabled,
                vessel_client::CallError::RequestIdsExhausted => Failure::Exhausted,
                vessel_client::CallError::Disconnected => effect.uncertain(Failure::Offline),
                vessel_client::CallError::UnexpectedReply(_) => {
                    effect.uncertain(Failure::Unexpected)
                }
            },
            #[cfg(target_arch = "wasm32")]
            Self::Call(_) => Failure::NoTransport,
            Self::Retired(why) => effect.uncertain(connection(*why)),
            Self::Deadline => effect.uncertain(Failure::TimedOut),
            Self::NotSelected(why) => connection(*why),
            Self::Offline => Failure::Offline,
            Self::Busy => Failure::Busy,
            Self::Preferences => Failure::Preferences,
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
