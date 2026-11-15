//! Selected-node diagnostics output is an allowlist boundary, including errors.
//! Never render an RPC/connection error's Display or cause chain for this command.
use crate::{args::Target, connect::connect};
mod error;
mod text;
pub(crate) use error::CommandError;
use std::fmt::{self, Write as _};
use vessel_proto::diagnostics::{DiagnosticCount, DiagnosticReport, DiagnosticSection};

/// Complete single-node text/JSON export budget, excluding shell framing.
const MAX_EXPORT_BYTES: usize = 128 * 1024;
#[derive(Debug, Clone, Copy)]
pub(crate) enum Output {
    Text,
    Json,
}

#[derive(Default)]
struct Text(String);
impl fmt::Write for Text {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self
            .0
            .len()
            .checked_add(value.len())
            .is_none_or(|len| len > MAX_EXPORT_BYTES)
        {
            return Err(fmt::Error);
        }
        self.0.push_str(value);
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub(crate) enum OutputError {
    #[error("diagnostic output exceeds {MAX_EXPORT_BYTES} bytes or formatting failed")]
    Format(#[from] fmt::Error),
    #[error("diagnostic JSON encoding failed")]
    Json(#[from] serde_json::Error),
}
pub(crate) async fn command(target: &Target, output: Output) -> Result<(), CommandError> {
    let client = connect(target)
        .await
        .map_err(|source| CommandError::Connection(Box::new(source)))?
        .client;
    let report = client.diagnostics().await?;
    let text = render(&report, output)?;
    crate::io::out(text.as_bytes())
        .await
        .map_err(CommandError::Write)
}
fn count(value: DiagnosticCount) -> String {
    if value.is_saturated() {
        format!("{}+ (saturated)", value.get())
    } else {
        value.get().to_string()
    }
}
fn section<T>(
    out: &mut Text,
    title: &str,
    section: &DiagnosticSection<T>,
    render: impl FnOnce(&mut Text, &T) -> fmt::Result,
) -> fmt::Result {
    writeln!(out, "{title}")?;
    match section {
        DiagnosticSection::Available { at, value } => {
            writeln!(out, "  Observed at: {at}")?;
            render(out, value)
        }
        DiagnosticSection::Unavailable { reason } => writeln!(out, "  Unavailable: {reason:?}"),
    }
}
fn render(report: &DiagnosticReport, output: Output) -> Result<String, OutputError> {
    match output {
        Output::Json => {
            let value = serde_json::to_string(report)?;
            if value.len() > MAX_EXPORT_BYTES {
                return Err(fmt::Error.into());
            }
            Ok(value)
        }
        Output::Text => Ok(text::report(report)?),
    }
}

#[cfg(test)]
mod tests;
