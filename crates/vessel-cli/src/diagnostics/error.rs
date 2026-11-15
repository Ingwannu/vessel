//! Diagnostic output errors retain typed private evidence behind a safe Display.
use crate::error::CliError;
use vessel_client::{CallError, DiagnosticError};
use vessel_proto::{ErrorCode, ErrorDetail, diagnostics::DiagnosticRefusal};

#[derive(Debug, thiserror::Error)]
pub(crate) enum CommandError {
    #[error("diagnostics unavailable: connection failed; source details omitted")]
    Connection(#[source] Box<CliError>),
    #[error("{}", request_message(.0))]
    Request(#[from] DiagnosticError),
    #[error("diagnostics unavailable: output could not be encoded within its byte budget")]
    Output(#[from] super::OutputError),
    #[error("diagnostics unavailable: output write failed")]
    Write(#[source] std::io::Error),
}
fn request_message(error: &DiagnosticError) -> &'static str {
    match error {
        DiagnosticError::Identity { .. } => "diagnostics refused: response belongs to another node",
        DiagnosticError::Timeout(_) => "diagnostics unavailable: response deadline expired",
        DiagnosticError::Call(error) => match error {
            CallError::UnassignedProtocol { .. } => {
                "diagnostics unavailable: protocol number is not assigned"
            }
            CallError::NotInProtocol { .. } => {
                "diagnostics unavailable: this peer uses an older protocol"
            }
            CallError::Disconnected => "diagnostics unavailable: node disconnected",
            CallError::UnexpectedReply(_) => {
                "diagnostics unavailable: peer returned an unexpected reply"
            }
            CallError::RequestIdsExhausted => {
                "diagnostics unavailable: reconnect before requesting another report"
            }
            CallError::Remote(error) => match error.code {
                ErrorCode::Forbidden => "diagnostics refused: Read scope is required",
                ErrorCode::Unauthorized => "diagnostics refused: authentication is required",
                ErrorCode::Unavailable => match error.detail {
                    Some(ErrorDetail::Diagnostics {
                        reason: DiagnosticRefusal::Busy,
                    }) => "diagnostics unavailable: collectors are busy",
                    Some(ErrorDetail::Diagnostics {
                        reason: DiagnosticRefusal::Stopping,
                    }) => "diagnostics unavailable: node is stopping",
                    Some(
                        ErrorDetail::DeviceInvite { .. }
                        | ErrorDetail::FleetControl { .. }
                        | ErrorDetail::SchedulePreview { .. }
                        | ErrorDetail::Browser { .. }
                        | ErrorDetail::Unknown,
                    )
                    | None => "diagnostics unavailable: node is overloaded or stopping",
                },
                ErrorCode::InvalidRequest
                | ErrorCode::ProtocolMismatch
                | ErrorCode::NotFound
                | ErrorCode::Conflict
                | ErrorCode::Unsupported
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Internal => "diagnostics unavailable: node refused the request",
            },
        },
    }
}
