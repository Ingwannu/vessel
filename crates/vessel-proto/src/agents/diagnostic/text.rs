//! An agent's diagnostic as Paseo's text (`diagnostic-utils.ts`
//! `formatProviderDiagnostic`): the title, then `  Label: value` rows, then
//! `  Models` and `  Status`. The page renders the same text in
//! TypeScript (`web/src/data/agent-diagnostic.ts`); the fixtures
//! `web/src/wire/fixtures/agent-diagnostic.{json,txt}` hold both to it.

use std::fmt::Write as _;

use super::{AgentDiagnostic, CatalogStatus, CommandSource, ProbeStep};
use crate::agents::{CheckStep, Installed, SignedIn, UnknownReason};
use crate::harness::StructuredSupport;

/// How a multi-line value's later lines are indented (Paseo's).
const CONTINUED: &str = "\n    ";

impl AgentDiagnostic {
    /// The diagnostic as Paseo's text, ending with a newline.
    #[must_use]
    pub fn text(&self) -> String {
        let mut rows: Vec<(&str, String)> = vec![
            ("Command source", source(self.command.source).to_owned()),
            ("Configured command", command(self)),
            ("Search path matches", matches(self)),
        ];
        installed(&self.installed, &mut rows);
        rows.push(("Auth", signed_in(&self.signed_in)));
        rows.push(("ACP spawn", step(&self.probe.spawn)));
        rows.push(("ACP initialize", initialize(self)));
        rows.push(("ACP session/new", step(&self.probe.session)));
        rows.push(("Probe took", format!("{} ms", self.probe.took.get())));
        let (models, status) = catalog(&self.status);
        rows.push(("Models", models));
        rows.push(("Status", status));
        let mut out = self.title.clone();
        out.push('\n');
        for (label, value) in rows {
            let _infallible = writeln!(out, "  {label}: {value}");
        }
        out
    }
}

const fn source(source: CommandSource) -> &'static str {
    match source {
        CommandSource::BuiltIn => "built in",
        CommandSource::Added => "added",
    }
}

fn command(diagnostic: &AgentDiagnostic) -> String {
    let mut words = vec![diagnostic.command.program.as_str()];
    words.extend(diagnostic.command.args.iter().map(String::as_str));
    words.join(" ")
}

fn matches(diagnostic: &AgentDiagnostic) -> String {
    if diagnostic.matches.is_empty() {
        return "none".to_owned();
    }
    let paths: Vec<String> = diagnostic
        .matches
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    paths.join(CONTINUED)
}

fn installed(installed: &Installed, rows: &mut Vec<(&str, String)>) {
    let version = |version: Option<&crate::agents::AgentVersion>| {
        version.map_or_else(|| "unknown".to_owned(), ToString::to_string)
    };
    match installed {
        Installed::Found {
            path,
            version: found,
            structured,
        } => {
            rows.push(("Resolved path", path.display().to_string()));
            rows.push(("Version", version(found.as_ref())));
            rows.push(("Conversations", conversations(structured)));
        }
        Installed::Missing { program } => {
            rows.push(("Resolved path", format!("not found ({program})")));
            rows.push(("Version", "unknown".to_owned()));
            rows.push(("Conversations", "unavailable: not installed".to_owned()));
        }
        Installed::TooOld {
            path,
            version: found,
            speaks,
            needs,
        }
        | Installed::TooNew {
            path,
            version: found,
            speaks,
            needs,
        } => {
            rows.push(("Resolved path", path.display().to_string()));
            rows.push(("Version", version(found.as_ref())));
            rows.push((
                "Conversations",
                format!("unavailable: it speaks ACP {speaks}, Vessel speaks {needs}"),
            ));
        }
        Installed::Unknown { reason } => {
            rows.push(("Resolved path", format!("unknown ({})", unknown(reason))));
            rows.push(("Version", "unknown".to_owned()));
            rows.push(("Conversations", "unknown".to_owned()));
        }
    }
}

fn conversations(structured: &StructuredSupport) -> String {
    match structured {
        StructuredSupport::Available => "available".to_owned(),
        StructuredSupport::Unavailable { reason } => format!("unavailable: {reason}"),
    }
}

fn signed_in(signed_in: &SignedIn) -> String {
    match signed_in {
        SignedIn::Yes => "signed in".to_owned(),
        SignedIn::No { .. } => "not signed in".to_owned(),
        SignedIn::NotRequired => "not required (a model route)".to_owned(),
        SignedIn::Unknown { reason } => format!("unknown ({})", unknown(reason)),
    }
}

fn unknown(reason: &UnknownReason) -> String {
    match reason {
        UnknownReason::NotInstalled => "not installed".to_owned(),
        UnknownReason::NotChecked => "not checked".to_owned(),
        UnknownReason::InUse => "in use by a session".to_owned(),
        UnknownReason::NotSaid => "the agent did not say".to_owned(),
        UnknownReason::NoHome => "no home for the profile".to_owned(),
        UnknownReason::Unread => "its answer was not read".to_owned(),
        UnknownReason::TimedOut { after } => format!("timed out after {} ms", after.get()),
        UnknownReason::Failed { step } => format!("failed at {}", check_step(*step)),
    }
}

const fn check_step(step: CheckStep) -> &'static str {
    match step {
        CheckStep::Spawn => "spawn",
        CheckStep::Initialize => "initialize",
        CheckStep::SessionNew => "session/new",
        CheckStep::Exited => "exit",
    }
}

fn step(step: &ProbeStep) -> String {
    match step {
        ProbeStep::Ok => "ok".to_owned(),
        ProbeStep::Failed { reason } => format!("error: {reason}"),
        ProbeStep::NotReached => "not reached".to_owned(),
    }
}

fn initialize(diagnostic: &AgentDiagnostic) -> String {
    match (
        &diagnostic.probe.initialize,
        &diagnostic.probe.agent_version,
    ) {
        (ProbeStep::Ok, Some(version)) => format!("ok (agent {version})"),
        (other, _) => step(other),
    }
}

fn catalog(status: &CatalogStatus) -> (String, String) {
    match status {
        CatalogStatus::Ready { models } => (models.to_string(), "Ready".to_owned()),
        CatalogStatus::Error { message } => ("\u{2014}".to_owned(), format!("Error: {message}")),
        CatalogStatus::Unavailable => ("\u{2014}".to_owned(), "Unavailable".to_owned()),
    }
}
