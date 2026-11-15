//! Paseo-style section/label text, bounded by the parent writer.
use super::{Text, count, section};
use std::fmt::{self, Write as _};
use vessel_proto::diagnostics::{
    DiagnosticAgentId, DiagnosticProcessId, DiagnosticReport, DiagnosticReportFields,
    DiagnosticVersion,
};

pub(super) fn report(report: &DiagnosticReport) -> Result<String, fmt::Error> {
    let fields = report.fields();
    let mut out = Text::default();
    header(&mut out, fields)?;
    process(&mut out, fields)?;
    store(&mut out, fields)?;
    agents(&mut out, fields)?;
    sessions(&mut out, fields)?;
    listeners(&mut out, fields)?;
    relays(&mut out, fields)?;
    failures(&mut out, fields)?;
    Ok(out.0)
}

fn version(value: Option<&DiagnosticVersion>) -> String {
    value.map_or_else(|| "unknown".to_owned(), ToString::to_string)
}

fn header(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    writeln!(
        out,
        "Vessel diagnostics\n  Collected at: {}\n  Node: {}\n  Version: {}\n  Platform: {:?} / {:?}\n  State: {:?}\n  Uptime: {} ms\n  Home: ~",
        fields.collected_at,
        fields.node.id,
        version(fields.node.version.as_ref()),
        fields.node.os,
        fields.node.arch,
        fields.node.state,
        fields.node.uptime.get()
    )
}

fn process(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(out, "Daemon process", &fields.process, |out, value| {
        writeln!(
            out,
            "  PID: {}\n  Parent: {:?}",
            value.pid.get(),
            value.parent.map(DiagnosticProcessId::get)
        )?;
        section(out, "  Memory", &value.memory, |out, memory| {
            writeln!(
                out,
                "    Resident bytes: {}\n    Virtual bytes: {}",
                count(memory.resident_bytes),
                count(memory.virtual_bytes)
            )
        })
    })
}

fn store(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(out, "Store", &fields.store, |out, value| {
        writeln!(
            out,
            "  Supported schema: {}\n  Observed schema: {}\n  Sessions: {} total / {} live / {} ended",
            value.expected_schema.get(),
            value.observed_schema.get(),
            count(value.sessions.total),
            count(value.sessions.live),
            count(value.sessions.ended)
        )
    })
}

fn agents(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(out, "Agents", &fields.agents, |out, values| {
        for value in values.as_slice() {
            let id = match value.id {
                DiagnosticAgentId::Builtin { harness } => harness.to_string(),
                DiagnosticAgentId::Custom { slot } => format!("Custom {}", slot.get()),
            };
            writeln!(
                out,
                "  {id}\n    Program: {:?}\n    Conversation: {:?}\n    Terminal: {:?}\n    Sign-in: {:?}\n    Cached version: {}\n    Cached at: {:?}\n    Stale: {}",
                value.executable,
                value.structured,
                value.terminal,
                value.sign_in,
                version(value.version.as_ref()),
                value.checked_at,
                value.stale
            )?;
        }
        Ok(())
    })
}

fn sessions(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(out, "Runtime sessions", &fields.sessions, |out, value| {
        writeln!(
            out,
            "  Live: {}\n  Parked: {}",
            count(value.live),
            count(value.parked)
        )
    })
}

fn listeners(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(out, "Listeners", &fields.listeners, |out, value| {
        writeln!(
            out,
            "  Local kind: {:?}\n  Local: {:?}\n  Direct: {:?}",
            value.local_kind, value.local, value.direct
        )
    })
}

fn relays(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(out, "Relays", &fields.relays, |out, value| {
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
    })
}

fn failures(out: &mut Text, fields: &DiagnosticReportFields) -> fmt::Result {
    section(
        out,
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
    )
}
