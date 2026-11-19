//! The node report as Paseo's text (`session/daemon/diagnostics.ts`: a
//! title, then `  Label: value` rows, sections apart by a blank line), for
//! every Rust client: the CLI (`vessel node diagnostics`) and the native
//! app's diagnostic (ADR 0111). The page renders the same text in
//! TypeScript (`web/src/data/node-diagnostics.ts`); the fixtures
//! `web/src/wire/fixtures/node-diagnostics.{json,txt}` hold both.
//!
//! [Decision Log]
//! - Purpose: one text for a report, whichever client shows or copies it,
//!   so a report pasted from the page reads as one from the app or the CLI.
//! - Constraint: two languages render it. Hand-written words per enum would
//!   be two tables to keep in step.
//! - Choice: a value's words follow one rule over its wire form ([`words`]):
//!   a closed value is its wire name with `_` read as a space; a tagged one
//!   is its tag, then its other fields in parentheses, by name. Labels,
//!   counts, versions and times have their own small rules below. The
//!   TypeScript renderer applies the same rules to the JSON it received.
//! - Cost: the words are the wire's names ("not checked", "too old"), not
//!   sentences; the report is for whoever reads it to someone who helps.

use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;

use super::{
    DiagnosticCount, DiagnosticMore, DiagnosticReport, DiagnosticReportFields, DiagnosticSection,
    DiagnosticTool, DiagnosticVersion,
};

/// The tags a tagged wire value names itself by, in the order looked for.
const TAGS: [&str; 3] = ["state", "reason", "kind"];

/// The report as Paseo's text, ending with a newline.
#[must_use]
pub fn report(report: &DiagnosticReport) -> String {
    let fields = report.fields();
    let mut sections = vec![
        header(fields),
        process(fields),
        store(fields),
        agents(fields),
    ];
    sections.push(section(
        "Runtime sessions",
        &fields.sessions,
        |rows, value| {
            rows.push(row("Live", count(value.live)));
            rows.push(row("Parked", count(value.parked)));
        },
    ));
    sections.push(section("Listeners", &fields.listeners, |rows, value| {
        rows.push(row("Local kind", words(&value.local_kind)));
        rows.push(row("Local", words(&value.local)));
        rows.push(row("Direct", words(&value.direct)));
    }));
    sections.push(relays(fields));
    sections.push(failures(fields));
    if let Some(more) = &fields.more {
        more_sections(more, &mut sections);
    }
    let mut out = sections.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn process(fields: &DiagnosticReportFields) -> String {
    section("Daemon process", &fields.process, |rows, value| {
        rows.push(row("PID", value.pid.get().to_string()));
        rows.push(row(
            "Parent",
            value
                .parent
                .map_or_else(|| "none".to_owned(), |parent| parent.get().to_string()),
        ));
        match &value.memory {
            DiagnosticSection::Available { value: memory, .. } => {
                rows.push(row("Resident bytes", count(memory.resident_bytes)));
                rows.push(row("Virtual bytes", count(memory.virtual_bytes)));
            }
            DiagnosticSection::Unavailable { reason } => {
                rows.push(row("Memory", format!("unavailable ({})", words(reason))));
            }
        }
    })
}

fn store(fields: &DiagnosticReportFields) -> String {
    section("Store", &fields.store, |rows, value| {
        rows.push(row(
            "Supported schema",
            value.expected_schema.get().to_string(),
        ));
        rows.push(row(
            "Observed schema",
            value.observed_schema.get().to_string(),
        ));
        rows.push(row(
            "Sessions",
            format!(
                "{} total / {} live / {} ended",
                count(value.sessions.total),
                count(value.sessions.live),
                count(value.sessions.ended)
            ),
        ));
    })
}

fn agents(fields: &DiagnosticReportFields) -> String {
    section("Agents", &fields.agents, |rows, values| {
        for agent in values.as_slice() {
            rows.push(format!("  {}", words(&agent.id)));
            rows.push(format!("    Program: {}", words(&agent.executable)));
            rows.push(format!("    Conversations: {}", words(&agent.structured)));
            rows.push(format!("    Terminal: {}", words(&agent.terminal)));
            rows.push(format!("    Sign-in: {}", words(&agent.sign_in)));
            rows.push(format!("    Version: {}", version(agent.version.as_ref())));
            rows.push(format!(
                "    Checked at: {}",
                agent
                    .checked_at
                    .map_or_else(|| "never".to_owned(), |at| raw(&at))
            ));
            rows.push(format!("    Stale: {}", words(&agent.stale)));
        }
    })
}

fn relays(fields: &DiagnosticReportFields) -> String {
    section("Relays", &fields.relays, |rows, value| {
        for (label, counts) in [
            ("Configured", &value.configured),
            ("Callback", &value.callback),
        ] {
            rows.push(row(
                label,
                format!(
                    "{} connecting / {} registered / {} refused / {} stopped / {} unknown",
                    count(counts.connecting),
                    count(counts.registered),
                    count(counts.refused),
                    count(counts.stopped),
                    count(counts.unrecognized)
                ),
            ));
        }
    })
}

fn failures(fields: &DiagnosticReportFields) -> String {
    section("Recent failures", &fields.recent_failures, |rows, value| {
        rows.push(row("Coverage", words(&value.coverage)));
        rows.push(row("Since", raw(&value.observed_since)));
        rows.push(row("Omitted", count(value.omitted)));
        for failure in value.entries.as_slice() {
            rows.push(row(&raw(&failure.at), words(&failure.code)));
        }
    })
}

/// ADR 0111's sections (Paseo's System, Disk, Workspaces, Agents, Tools).
fn more_sections(more: &DiagnosticMore, sections: &mut Vec<String>) {
    sections.push(section("System", &more.system, |rows, value| {
        rows.push(row("CPU cores", count(value.cpus)));
        rows.push(row(
            "Load avg",
            value.load.map_or_else(
                || "unsupported".to_owned(),
                |load| {
                    format!(
                        "{} {} {}",
                        hundredths(load.one),
                        hundredths(load.five),
                        hundredths(load.fifteen)
                    )
                },
            ),
        ));
        rows.push(row(
            "Memory available",
            value.memory.map_or_else(
                || "unsupported".to_owned(),
                |memory| {
                    format!(
                        "{} of {} bytes",
                        count(memory.available_bytes),
                        count(memory.total_bytes)
                    )
                },
            ),
        ));
    }));
    sections.push(section("Disk", &more.disk, |rows, value| {
        rows.push(row("Data directory", "~".to_owned()));
        rows.push(row(
            "Available",
            format!(
                "{} of {} bytes",
                count(value.available_bytes),
                count(value.total_bytes)
            ),
        ));
    }));
    sections.push(section("Workspaces", &more.workspaces, |rows, value| {
        rows.push(row("Projects", count(value.projects)));
        rows.push(row(
            "Workspaces",
            format!(
                "{} active ({} worktrees) / {} archived",
                count(value.active),
                count(value.managed),
                count(value.archived)
            ),
        ));
    }));
    sections.push(section("Activity", &more.activity, |rows, value| {
        let by_agent: Vec<String> = value
            .builtin
            .iter()
            .map(|agent| format!("{} {}", words(&agent.harness), count(agent.live)))
            .chain((value.added.get() > 0).then(|| format!("added {}", count(value.added))))
            .collect();
        rows.push(row(
            "Live by agent",
            if by_agent.is_empty() {
                "none".to_owned()
            } else {
                by_agent.join(", ")
            },
        ));
        rows.push(row("Waiting on a permission", count(value.waiting)));
    }));
    sections.push(section("Tools", &more.tools, |rows, value| {
        rows.push(row("git", tool(&value.git)));
        rows.push(row("gh", tool(&value.gh)));
    }));
}

fn header(fields: &DiagnosticReportFields) -> String {
    let node = &fields.node;
    let rows = [
        row("Collected at", raw(&fields.collected_at)),
        row("Node", raw(&node.id)),
        row("Version", version(node.version.as_ref())),
        row(
            "Platform",
            // An architecture's name keeps its `_` (`x86_64`).
            format!("{} / {}", words(&node.os), raw(&node.arch)),
        ),
        row("State", words(&node.state)),
        row("Uptime", format!("{} ms", node.uptime.get())),
        row("Home", "~".to_owned()),
    ];
    let mut out = String::from("Vessel diagnostics\n");
    for line in rows {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// A section: its title, when it was observed and its rows, or why it is
/// unavailable.
fn section<T>(
    title: &str,
    section: &DiagnosticSection<T>,
    fill: impl FnOnce(&mut Vec<String>, &T),
) -> String {
    let mut rows = Vec::new();
    match section {
        DiagnosticSection::Available { at, value } => {
            rows.push(row("Observed at", raw(at)));
            fill(&mut rows, value);
        }
        DiagnosticSection::Unavailable { reason } => {
            rows.push(row("Unavailable", words(reason)));
        }
    }
    let mut out = String::from(title);
    out.push('\n');
    for line in rows {
        let _infallible = writeln!(out, "{line}");
    }
    out
}

fn row(label: &str, mut value: String) -> String {
    value.insert_str(0, &format!("  {label}: "));
    value
}

/// A count, saying when it saturated.
fn count(value: DiagnosticCount) -> String {
    if value.is_saturated() {
        format!("{}+ (saturated)", value.get())
    } else {
        value.get().to_string()
    }
}

fn version(value: Option<&DiagnosticVersion>) -> String {
    value.map_or_else(|| "unknown".to_owned(), ToString::to_string)
}

fn tool(tool: &DiagnosticTool) -> String {
    match tool {
        DiagnosticTool::Found { version: found } => {
            format!("found, version {}", version(found.as_ref()))
        }
        DiagnosticTool::Missing | DiagnosticTool::TimedOut | DiagnosticTool::Failed => words(tool),
    }
}

/// Hundredths as a decimal (`42` is `0.42`).
fn hundredths(value: u32) -> String {
    let whole = value.checked_div(100).unwrap_or_default();
    let part = value.checked_rem(100).unwrap_or_default();
    format!("{whole}.{part:02}")
}

/// A value's wire form as text, unchanged: a string as it is, anything
/// else as its JSON (times and ids, whose `_` must stay).
fn raw<T: Serialize + ?Sized>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => text,
        Ok(other) => other.to_string(),
        Err(_unwritable) => "unreadable".to_owned(),
    }
}

/// A value in words, by the rule both renderers follow: `null` is "none",
/// a boolean "yes" or "no", a string its text with `_` read as a space, a
/// list its items joined by ", ", and an object its tag ([`TAGS`]) then its
/// other fields as "name: words", sorted by name, in parentheses (or alone
/// when it has no tag).
#[must_use]
pub fn words<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_value(value).map_or_else(
        |_unwritable| "unreadable".to_owned(),
        |value| json_words(&value),
    )
}

fn json_words(value: &Value) -> String {
    match value {
        Value::Null => "none".to_owned(),
        Value::Bool(true) => "yes".to_owned(),
        Value::Bool(false) => "no".to_owned(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.replace('_', " "),
        Value::Array(items) => items.iter().map(json_words).collect::<Vec<_>>().join(", "),
        Value::Object(map) => {
            let tag = TAGS.iter().find_map(|key| {
                map.get(*key)
                    .and_then(Value::as_str)
                    .map(|text| (*key, text))
            });
            let mut rest: Vec<(&String, &Value)> = map
                .iter()
                .filter(|(key, _value)| tag.is_none_or(|(tag_key, _text)| key.as_str() != tag_key))
                .collect();
            rest.sort_by(|left, right| left.0.cmp(right.0));
            let fields: Vec<String> = rest
                .into_iter()
                .map(|(key, value)| format!("{}: {}", key.replace('_', " "), json_words(value)))
                .collect();
            match tag {
                Some((_key, text)) if fields.is_empty() => text.replace('_', " "),
                Some((_key, text)) => format!("{} ({})", text.replace('_', " "), fields.join(", ")),
                None => fields.join(", "),
            }
        }
    }
}

#[cfg(test)]
mod tests;
