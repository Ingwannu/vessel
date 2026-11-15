//! One bounded read of the selected machine's existing model/profile lists.
//! No agent process is started and no configuration value is changed.
use vessel_app_view::agent_status::pages::MachineKey;
use vessel_app_view::models::{Problem, page::Snapshot};
use vessel_proto::RequestId;

/// The machine and never-reused view ticket travel together.
#[derive(Debug)]
pub struct Ask {
    /// Validated book route.
    pub machine: MachineKey,
    /// Current Models page read.
    pub ticket: RequestId,
}
/// Complete snapshot or typed failure from that read.
#[derive(Debug)]
pub struct Answer {
    /// Original book route, never the currently visible machine.
    pub machine: MachineKey,
    /// Original read identity.
    pub ticket: RequestId,
    /// All facts came from the same connection.
    pub result: Result<Snapshot, ModelsError>,
}
impl Ask {
    /// A command rejected before dispatch must settle its waiting page too.
    #[must_use]
    pub fn refused(self, error: ModelsError) -> Answer {
        Answer {
            machine: self.machine,
            ticket: self.ticket,
            result: Err(error),
        }
    }
}
/// Original errors stay available while only a translated meaning is drawn.
#[derive(Debug, thiserror::Error)]
pub enum ModelsError {
    /// No current connection exists.
    #[error("the machine is not connected")]
    Offline,
    /// Read capacity or the shared command queue is occupied.
    #[error("the model request could not be queued")]
    Busy,
    /// No complete snapshot arrived before the read budget.
    #[error("the model read exceeded its deadline")]
    Timeout,
    /// No browser transport exists in this build.
    #[error("this browser has no node transport")]
    NoTransport,
    /// Preserve the typed wire code and diagnostic, not their prose in UI.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Call(#[from] vessel_client::CallError),
}
impl ModelsError {
    /// Plain-language category; never translates arbitrary machine prose.
    #[must_use]
    pub const fn problem(&self) -> Problem {
        match self {
            Self::Offline => Problem::Offline,
            Self::Busy => Problem::Busy,
            Self::Timeout => Problem::Timeout,
            Self::NoTransport => Problem::NoTransport,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(error) => match error {
                vessel_client::CallError::Disconnected => Problem::Offline,
                vessel_client::CallError::NotInProtocol { .. } => Problem::Unsupported,
                vessel_client::CallError::RequestIdsExhausted
                | vessel_client::CallError::UnexpectedReply(_) => Problem::Failed,
                vessel_client::CallError::Remote(error) => match error.code {
                    vessel_proto::ErrorCode::Forbidden | vessel_proto::ErrorCode::Unauthorized => {
                        Problem::Denied
                    }
                    vessel_proto::ErrorCode::Unsupported
                    | vessel_proto::ErrorCode::ProtocolMismatch => Problem::Unsupported,
                    vessel_proto::ErrorCode::InvalidRequest
                    | vessel_proto::ErrorCode::NotFound
                    | vessel_proto::ErrorCode::Conflict
                    | vessel_proto::ErrorCode::HarnessNotInstalled
                    | vessel_proto::ErrorCode::SpawnFailed
                    | vessel_proto::ErrorCode::Unavailable
                    | vessel_proto::ErrorCode::Internal => Problem::Failed,
                },
            },
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::Hub;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Feature-local connection lookup keeps root dispatch a forwarding arm.
    pub(super) async fn read_models(&self, ask: Ask) {
        let client = self
            .clients
            .get(ask.machine.as_str())
            .map(|connected| connected.client.clone());
        self.models.ask(client, ask, self.events.clone()).await;
    }
}
