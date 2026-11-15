//! Selected-node diagnostics output is an allowlist boundary, including errors.
//! Never render an RPC/connection error's Display or cause chain for this command.
use crate::{args::Target, connect::connect};
mod error;
pub(crate) use error::CommandError;
use std::fmt::{self, Write as _};
use vessel_proto::diagnostics::{
    DiagnosticAgentId, DiagnosticCount, DiagnosticReport, DiagnosticSection,
};

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
    if let Output::Json = output {
        let value = serde_json::to_string(report)?;
        if value.len() > MAX_EXPORT_BYTES {
            return Err(fmt::Error.into());
        }
        return Ok(value);
    }
    let fields = report.fields();
    let mut out = Text::default();
    writeln!(
        out,
        "Vessel diagnostics\n  Collected at: {}\n  Node: {}\n  Version: {:?}\n  Platform: {:?} / {:?}\n  State: {:?}\n  Uptime: {:?}\n  Home: ~",
        fields.collected_at,
        fields.node.id,
        fields.node.version,
        fields.node.os,
        fields.node.arch,
        fields.node.state,
        fields.node.uptime
    )?;
    section(&mut out, "Daemon process", &fields.process, |out, value| {
        writeln!(
            out,
            "  PID: {}\n  Parent: {:?}",
            value.pid.get(),
            value.parent.map(|id| id.get())
        )?;
        section(out, "  Memory", &value.memory, |out, memory| {
            writeln!(
                out,
                "    Resident bytes: {}\n    Virtual bytes: {}",
                count(memory.resident_bytes),
                count(memory.virtual_bytes)
            )
        })
    })?;
    section(&mut out, "Store", &fields.store, |out, value| {
        writeln!(
            out,
            "  Supported schema: {:?}\n  Observed schema: {:?}\n  Sessions: {} total / {} live / {} ended",
            value.expected_schema,
            value.observed_schema,
            count(value.sessions.total),
            count(value.sessions.live),
            count(value.sessions.ended)
        )
    })?;
    section(&mut out, "Agents", &fields.agents, |out, values| {
        for value in values.as_slice() {
            let id = match value.id {
                DiagnosticAgentId::Builtin { harness } => harness.to_string(),
                DiagnosticAgentId::Custom { slot } => format!("Custom {}", slot.get()),
            };
            writeln!(
                out,
                "  {id}\n    Program: {:?}\n    Conversation: {:?}\n    Terminal: {:?}\n    Sign-in: {:?}\n    Cached version: {:?}\n    Cached at: {:?}\n    Stale: {}",
                value.executable,
                value.structured,
                value.terminal,
                value.sign_in,
                value.version,
                value.checked_at,
                value.stale
            )?;
        }
        Ok(())
    })?;
    section(
        &mut out,
        "Runtime sessions",
        &fields.sessions,
        |out, value| {
            writeln!(
                out,
                "  Live: {}\n  Parked: {}",
                count(value.live),
                count(value.parked)
            )
        },
    )?;
    section(&mut out, "Listeners", &fields.listeners, |out, value| {
        writeln!(
            out,
            "  Local kind: {:?}\n  Local: {:?}\n  Direct: {:?}",
            value.local_kind, value.local, value.direct
        )
    })?;
    section(&mut out, "Relays", &fields.relays, |out, value| {
        for (label, counts) in [
            ("Configured", &value.configured),
            ("Callback", &value.callback),
        ] {
            writeln!(
                out,
                "  {label}: {} connecting / {} registered / {} refused / {} stopped / {} unknown",
                count(counts.connecting),
                count(counts.registered),
                count(counts.refused),
                count(counts.stopped),
                count(counts.unrecognized)
            )?;
        }
        Ok(())
    })?;
    section(
        &mut out,
        "Recent failures",
        &fields.recent_failures,
        |out, value| {
            writeln!(
                out,
                "  Coverage: {:?}\n  Since: {}\n  Omitted: {}",
                value.coverage,
                value.observed_since,
                count(value.omitted)
            )?;
            for failure in value.entries.as_slice() {
                writeln!(out, "  {}: {:?}", failure.at, failure.code)?;
            }
            Ok(())
        },
    )?;
    Ok(out.0)
}

#[cfg(test)]
mod tests;
