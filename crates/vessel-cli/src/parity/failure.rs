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
    let code = if matches!(
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
    } else if matches!(error, CliError::NoSuchSession(_)) {
        vessel_proto::ErrorCode::NotFound
    } else if let CliError::Call(vessel_client::CallError::Remote(failure)) = error {
        failure.code
    } else {
        vessel_proto::ErrorCode::Internal
    };
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
