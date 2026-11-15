//! The app diagnostic of Settings › Diagnostics, as Paseo's
//! (`diagnostics/app-diagnostic-report.ts`, `components/app-diagnostic-sheet.tsx`):
//! what the app knows about itself and each machine in the book, as one
//! plain text a person copies into a report.
//!
//! [Decision Log]
//! - Purpose: Paseo's "App diagnostic" (Run, then a sheet with the text,
//!   Copy and Refresh): a person asked "what does your app say" pastes
//!   one block.
//! - Constraint: it is shared outside (a chat, an issue), so nothing in it
//!   may let someone reach or impersonate a machine: no addresses, keys,
//!   tokens or pairing links ([`redact`], as Paseo's
//!   `redactAppDiagnosticReport`).
//! - Choice: Paseo's text shape ("Title" then "  Label: value" lines,
//!   sections apart by a blank line) and its English labels: the text is
//!   for whoever reads the report, not part of the interface, so it is
//!   not translated (Paseo's is not either).
//! - What Vessel knows differs from Paseo's: per machine its reach, the
//!   protocol it speaks, its sessions and its agents with why one cannot
//!   start (Paseo: connection probes and the daemon's server info); for
//!   the app, its build, platform, renderer and where it keeps its
//!   preferences and log (Paseo: version, platform, desktop flag).
//! - Cost: an agent's "unavailable" sentence is the node's own words, kept
//!   whole after redaction; a sentence naming a path keeps the path.

use crate::board::{Machine, Reach};

/// The app's side of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppFacts {
    /// When it was collected (RFC 3339, UTC).
    pub collected_at: String,
    /// The build's label ("0.0.0.0.16", or "dev").
    pub label: String,
    /// The operating system and architecture (`linux x86_64`).
    pub platform: String,
    /// How the window is drawn.
    pub renderer: Renderer,
    /// Where the preferences are kept, in words (a file's path, or why
    /// for this launch only).
    pub preferences: String,
    /// The log's folder, if the app has one.
    pub log: Option<String>,
    /// The person's home folder, shown as `~` wherever a path starts with
    /// it: the report is pasted elsewhere, and an account's name in a path
    /// says nothing about the problem.
    pub home: Option<String>,
}

/// How the window is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Renderer {
    /// Not known yet (no frame drawn).
    Unknown,
    /// A GPU: its device and driver, as the graphics layer names them.
    Gpu {
        /// The device.
        device: String,
        /// The driver.
        driver: String,
    },
    /// A software renderer on the CPU (motion is held still).
    Software {
        /// The device (llvmpipe, lavapipe, WARP).
        device: String,
    },
}

/// The whole report: the app, then each machine in the book's order.
#[must_use]
pub fn report(app: &AppFacts, machines: &[Machine]) -> String {
    let mut sections = vec![app_section(app, machines.len())];
    sections.extend(machines.iter().map(machine_section));
    let text = sections.join("\n\n");
    // A home of "/" (or none) would turn every path into "~…": left alone.
    let text = match app
        .home
        .as_deref()
        .map(|home| home.trim_end_matches(['/', '\\']))
    {
        Some(home) if !home.is_empty() => text.replace(home, "~"),
        Some(_) | None => text,
    };
    redact(&text)
}

/// One section: its title, then a line per entry.
fn section(title: &str, entries: &[(&str, String)]) -> String {
    let mut text = title.to_owned();
    for (label, value) in entries {
        text.push_str("\n  ");
        text.push_str(label);
        text.push_str(": ");
        text.push_str(value);
    }
    text
}

fn app_section(app: &AppFacts, machines: usize) -> String {
    let renderer = match &app.renderer {
        Renderer::Unknown => "unknown".to_owned(),
        Renderer::Gpu { device, driver } => format!("GPU, {device} ({driver})"),
        Renderer::Software { device } => format!("software, {device} (motion held)"),
    };
    section(
        "Vessel app diagnostics",
        &[
            ("Collected at", app.collected_at.clone()),
            ("App version", app.label.clone()),
            ("Platform", app.platform.clone()),
            ("Renderer", renderer),
            ("Saved machines", machines.to_string()),
            ("Preferences", app.preferences.clone()),
            (
                "Log folder",
                app.log.clone().unwrap_or_else(|| "none".to_owned()),
            ),
        ],
    )
}

fn machine_section(machine: &Machine) -> String {
    let status = match &machine.reach {
        Reach::Online => "online".to_owned(),
        Reach::Unreachable(why) => format!("unreachable: {why}"),
    };
    let protocol = machine
        .protocol
        .map_or_else(|| "unknown".to_owned(), |protocol| protocol.to_string());
    let version = machine
        .version
        .as_ref()
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);
    let mut entries = vec![
        ("Status", status),
        ("Version", version),
        ("Protocol", protocol),
        ("Sessions", machine.sessions.len().to_string()),
    ];
    entries.extend(machine.agents.iter().map(|agent| {
        let state = agent.unavailable.as_ref().map_or_else(
            || "available".to_owned(),
            |why| format!("unavailable: {why}"),
        );
        ("Agent", format!("{}, {state}", agent.name))
    }));
    section(&format!("Machine: {}", machine.name), &entries)
}

/// Schemes whose addresses are taken out: a machine's routes, a relay's,
/// a pairing link.
const SCHEMES: [&str; 6] = [
    "ws://",
    "wss://",
    "http://",
    "https://",
    "vessel://",
    "ssh://",
];

/// Words after which the value is a secret, as Paseo's redaction names
/// them.
const SECRET_KEYS: [&str; 6] = [
    "token",
    "secret",
    "password",
    "key",
    "authorization",
    "credential",
];

/// The text with every address, key and secret value replaced by
/// `[redacted]`: a URL of [`SCHEMES`] to its end, a `name=value` or
/// `name: value` whose name holds one of [`SECRET_KEYS`], and an `x25519:`
/// key.
#[must_use]
pub fn redact(text: &str) -> String {
    text.split('\n')
        .map(redact_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut words = line.split(' ');
    let mut first = true;
    while let Some(word) = words.next() {
        if !first {
            out.push(' ');
        }
        first = false;
        out.push_str(&redact_word(word));
        // "token: value": the value is the next word.
        let lower = word.to_ascii_lowercase();
        if lower.ends_with(':') && names_a_secret(&lower) && words.next().is_some() {
            out.push_str(" [redacted]");
        }
    }
    out
}

fn redact_word(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    if let Some(at) = SCHEMES.iter().filter_map(|scheme| lower.find(scheme)).min() {
        // Keep what precedes the address ("(" or a label), drop the rest.
        let kept = word.get(..at).unwrap_or_default();
        return format!("{kept}[redacted]");
    }
    if let Some(at) = lower.find("x25519:") {
        let kept = word.get(..at).unwrap_or_default();
        return format!("{kept}x25519:[redacted]");
    }
    if let Some((name, _value)) = word.split_once('=')
        && names_a_secret(&name.to_ascii_lowercase())
    {
        return format!("{name}=[redacted]");
    }
    word.to_owned()
}

fn names_a_secret(name: &str) -> bool {
    SECRET_KEYS.iter().any(|key| name.contains(key))
}

#[cfg(test)]
#[path = "diagnostics/words_tests.rs"]
mod words_tests;
