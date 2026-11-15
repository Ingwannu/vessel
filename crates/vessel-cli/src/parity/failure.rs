//! Runtime error classification stays separate from reusable typed data output.

use crate::{args::parity::Presentation, error::CliError, io};
use serde::Serialize;

/// Runtime errors preserve a stable envelope instead of corrupting structured stdout.
pub(crate) async fn error(presentation: Presentation, error: &CliError) -> std::io::Result<()> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        error: Failure<'a>,
    }
    #[derive(Serialize)]
    struct Failure<'a> {
        code: vessel_proto::ErrorCode,
        message: &'a str,
    }
    let code = code(error);
    let message = crate::chain(error);
    match presentation {
        Presentation::Json | Presentation::Yaml => {
            let envelope = Envelope {
                error: Failure {
                    code,
                    message: &message,
                },
            };
            let text = serde_json::to_string(&envelope).map_err(std::io::Error::other)?;
            let prefix = if presentation == Presentation::Yaml {
                "---\n"
            } else {
                ""
            };
            io::note(&format!("{prefix}{text}\n")).await
        }
        Presentation::Quiet | Presentation::Table(_) => {
            io::note(&format!("error: {message}\n")).await
        }
    }
}

/// SDK admission failures have stable codes even before any remote frame exists.
fn code(error: &CliError) -> vessel_proto::ErrorCode {
    if matches!(
        error,
        CliError::ParityBlocked(_) | CliError::UnsupportedPresentation
    ) {
        vessel_proto::ErrorCode::Unsupported
    } else if matches!(
        error,
        CliError::Usage(_)
            | CliError::ParityLabel(_)
            | CliError::ParityHostText(_)
            | CliError::ParityCron(_)
            | CliError::ParityAutomationText(_)
            | CliError::ParityCloneUrl(_)
    ) {
        vessel_proto::ErrorCode::InvalidRequest
    } else if matches!(
        error,
        CliError::ParitySnapshotTimeout(_)
            | CliError::ParityPromptReadTimeout(_)
            | CliError::ParityCloneTimeout { .. }
            | CliError::ParityImageTimeout { .. }
    ) {
        vessel_proto::ErrorCode::Unavailable
    } else if matches!(error, CliError::ParityHostObservationChanged(_)) {
        vessel_proto::ErrorCode::Conflict
    } else if matches!(error, CliError::NoSuchSession(_)) {
        vessel_proto::ErrorCode::NotFound
    } else if let CliError::Call(failure)
    | CliError::Stream(vessel_client::StreamError::Call(failure)) = error
    {
        failure.code().unwrap_or(vessel_proto::ErrorCode::Internal)
    } else {
        vessel_proto::ErrorCode::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledged_mutation_with_a_later_observation_conflict_has_an_explicit_code() {
        let id = vessel_proto::host_registry::HostRegistrationId::random();
        assert_eq!(
            code(&CliError::ParityHostObservationChanged(id)),
            vessel_proto::ErrorCode::Conflict
        );
    }
    #[test]
    fn pending_and_older_sdk_preflight_keep_unsupported_in_cli_errors() {
        use vessel_client::CallError;
        use vessel_proto::{ErrorCode, Method};
        let pending = CliError::Call(CallError::UnassignedProtocol {
            method: Method::CaptureTerminal,
        });
        assert_eq!(code(&pending), ErrorCode::Unsupported);
        let older = CliError::Call(CallError::NotInProtocol {
            method: Method::CreateSession,
            needs: vessel_proto::PROTOCOL_VERSION,
            negotiated: vessel_proto::MIN_PROTOCOL_VERSION,
        });
        assert_eq!(code(&older), ErrorCode::Unsupported);
        assert_eq!(
            code(&CliError::Usage("invalid request")),
            ErrorCode::InvalidRequest
        );
    }
    #[test]
    fn stream_rpc_failures_keep_the_same_code_as_direct_calls() {
        use vessel_client::{CallError, StreamError};
        use vessel_proto::{ErrorCode, Method, WireError};
        for expected in [
            ErrorCode::Unsupported,
            ErrorCode::Forbidden,
            ErrorCode::NotFound,
        ] {
            let rpc = CallError::Remote(WireError::new(expected, "refused".to_owned()));
            for error in [
                CliError::Call(rpc.clone()),
                CliError::Stream(StreamError::Call(rpc)),
            ] {
                assert_eq!(code(&error), expected);
            }
        }
        let pending = CliError::Stream(StreamError::Call(CallError::UnassignedProtocol {
            method: Method::CaptureTerminal,
        }));
        assert_eq!(code(&pending), ErrorCode::Unsupported);
    }
}
