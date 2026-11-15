//! Settings > Agents (the page's `data/agent-status.ts` and the parts of
//! `agents.ts` it uses): every agent a machine knows, found or not, its
//! state in a headline, and what it offers a new session.
//!
//! [Decision Log]
//! - Purpose: Settings lists the four agents Vessel knows on every
//!   machine, found or not, beside the ones a person added (ADR 0032), so
//!   a missing one says it is missing rather than not being there.
//! - Choice: the page's order (the four found, then the added found, the
//!   added not found, then the four not found) and headlines; names come
//!   from the lead's [`crate::agent_name`], option rows from
//!   [`crate::controls_of`].

use chrono::{DateTime, Utc};
use vessel_proto::agents::{AgentDetail, AgentInfo};
use vessel_proto::{Harness, HarnessKind, HarnessRef, StructuredSupport, TerminalSupport};

use crate::ago::Ago;
use crate::machines::StateTone;
use crate::{Controls, Lang, SelectControl, agent_name};
pub mod command;
pub mod install;
pub mod notices;
pub mod pages;
pub mod problem;
pub mod readiness;
pub mod removal;
pub mod row;

/// An agent's key in an address or a list: its kind as the wire writes it
/// (`open-code`, where the CLI says `opencode`), or `added:<name>`.
#[must_use]
pub fn agent_key(agent: &HarnessRef) -> String {
    match agent {
        HarnessRef::Known(kind) => wire_name(*kind).to_owned(),
        HarnessRef::Added(name) => format!("added:{name}"),
    }
}

const fn wire_name(kind: HarnessKind) -> &'static str {
    match kind {
        HarnessKind::ClaudeCode => "claude-code",
        HarnessKind::Codex => "codex",
        HarnessKind::Gemini => "gemini",
        HarnessKind::OpenCode => "open-code",
    }
}

/// Which ways an installed agent runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentModes {
    /// As a conversation and in a terminal.
    Both,
    /// As a conversation only; why no terminal.
    Timeline(String),
    /// In a terminal only; why no conversation.
    Terminal(String),
    /// Neither; why.
    Neither(String),
}

impl AgentModes {
    /// The ways `harness` runs.
    #[must_use]
    pub fn of(harness: &Harness) -> Self {
        match (&harness.structured, &harness.terminal) {
            (StructuredSupport::Available, TerminalSupport::Available) => Self::Both,
            (StructuredSupport::Available, TerminalSupport::Unavailable { reason, .. }) => {
                Self::Timeline(reason.clone())
            }
            (StructuredSupport::Unavailable { reason }, TerminalSupport::Available) => {
                Self::Terminal(reason.clone())
            }
            (StructuredSupport::Unavailable { reason }, TerminalSupport::Unavailable { .. }) => {
                Self::Neither(reason.clone())
            }
        }
    }
}

/// An agent's state on one machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus<'a> {
    /// Found, and runs as a conversation.
    Available(&'a Harness),
    /// Found, but runs in a terminal only; the machine's reason.
    TerminalOnly {
        /// What the machine found.
        harness: &'a Harness,
        /// Why not as a conversation.
        reason: String,
    },
    /// Not found; the program an added agent runs, if it is one.
    NotFound {
        /// The added agent's program.
        program: Option<String>,
    },
}

impl AgentStatus<'_> {
    /// Whether the machine found it.
    #[must_use]
    pub const fn found(&self) -> bool {
        match self {
            Self::Available(_) | Self::TerminalOnly { .. } => true,
            Self::NotFound { .. } => false,
        }
    }
}

/// The state of `agent` among `harnesses`; `program` is an added agent's.
#[must_use]
pub fn agent_status<'a>(
    agent: &HarnessRef,
    harnesses: &'a [Harness],
    program: Option<&str>,
) -> AgentStatus<'a> {
    let Some(harness) = harnesses.iter().find(|found| found.kind == *agent) else {
        return AgentStatus::NotFound {
            program: program.map(str::to_owned),
        };
    };
    match &harness.structured {
        StructuredSupport::Available => AgentStatus::Available(harness),
        StructuredSupport::Unavailable { reason, .. } => AgentStatus::TerminalOnly {
            harness,
            reason: reason.clone(),
        },
    }
}

/// An agent's state as its row says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headline {
    /// One or two words.
    pub headline: &'static str,
    /// Its colour.
    pub tone: StateTone,
    /// What more there is to say.
    pub detail: Option<String>,
}

/// The headline of `status`.
#[must_use]
pub fn agent_headline(lang: Lang, status: &AgentStatus<'_>) -> Headline {
    let words = lang.machines();
    match status {
        AgentStatus::Available(harness) => Headline {
            headline: words.available,
            tone: StateTone::Ok,
            detail: matches!(AgentModes::of(harness), AgentModes::Timeline(_))
                .then(|| words.no_terminal.to_owned()),
        },
        AgentStatus::TerminalOnly { reason, .. } => Headline {
            headline: words.needs_attention,
            tone: StateTone::Warn,
            detail: Some(words.terminal_only(reason)),
        },
        AgentStatus::NotFound { program } => Headline {
            headline: words.not_found,
            tone: StateTone::Neutral,
            detail: Some(words.program_missing(program.as_deref())),
        },
    }
}

/// One agent in Settings' list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentOn<'a> {
    /// Which.
    pub agent: HarnessRef,
    /// Its state.
    pub status: AgentStatus<'a>,
}

/// Every agent on a machine: the four Vessel knows that were found, the
/// added ones found, the added ones not found, then the four not found.
/// `agents` is the machine's `agent.list` (empty before it is read: the
/// added agents found are still listed).
#[must_use]
pub fn agents_on<'a>(harnesses: &'a [Harness], agents: &[AgentInfo]) -> Vec<AgentOn<'a>> {
    let four: Vec<AgentOn<'a>> = HarnessKind::ALL
        .into_iter()
        .map(|kind| {
            let agent = HarnessRef::Known(kind);
            let status = agent_status(&agent, harnesses, None);
            AgentOn { agent, status }
        })
        .collect();
    let listed = agents.iter().filter_map(|info| match &info.detail {
        AgentDetail::Added { added, .. } => Some(AgentOn {
            agent: info.agent.clone(),
            status: agent_status(&info.agent, harnesses, Some(added.program.as_str())),
        }),
        AgentDetail::Known { .. } => None,
    });
    let mut added: Vec<AgentOn<'a>> = listed.collect();
    let found: Vec<AgentOn<'a>> = harnesses
        .iter()
        .filter(|harness| {
            harness.kind.known().is_none() && !added.iter().any(|entry| entry.agent == harness.kind)
        })
        .map(|harness| AgentOn {
            agent: harness.kind.clone(),
            status: agent_status(&harness.kind, harnesses, None),
        })
        .collect();
    added.extend(found);
    let (four_found, four_missing): (Vec<_>, Vec<_>) =
        four.into_iter().partition(|entry| entry.status.found());
    let (added_found, added_missing): (Vec<_>, Vec<_>) =
        added.into_iter().partition(|entry| entry.status.found());
    four_found
        .into_iter()
        .chain(added_found)
        .chain(added_missing)
        .chain(four_missing)
        .collect()
}

/// One option an agent offers a new session, as Settings lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferRow {
    /// The option's id.
    pub id: String,
    /// Its name.
    pub label: String,
    /// Its values' names.
    pub values: Vec<String>,
    /// The current value's name.
    pub current: String,
}

fn select_row(control: &SelectControl, label: &str) -> OfferRow {
    OfferRow {
        id: control.id.as_str().to_owned(),
        label: label.to_owned(),
        values: control
            .values
            .iter()
            .map(|value| value.name.clone())
            .collect(),
        current: control.current_name().to_owned(),
    }
}

/// What an agent offers: its model, effort and permissions in the app's
/// words, then its toggles, then its other choices by their own names.
#[must_use]
pub fn offer_rows(lang: Lang, controls: &Controls) -> Vec<OfferRow> {
    let slots = lang.options();
    let words = lang.machines();
    let named = [
        (&controls.model, slots.model),
        (&controls.effort, slots.effort),
        (&controls.permissions, slots.permissions),
    ];
    named
        .into_iter()
        .filter_map(|(control, label)| control.as_ref().map(|control| select_row(control, label)))
        .chain(controls.toggles.iter().map(|toggle| OfferRow {
            id: toggle.id.as_str().to_owned(),
            label: toggle.name.clone(),
            values: vec![words.on.to_owned(), words.off.to_owned()],
            current: if toggle.on { words.on } else { words.off }.to_owned(),
        }))
        .chain(
            controls
                .selects
                .iter()
                .map(|control| select_row(control, &control.name)),
        )
        .collect()
}

/// When the machine last asked its agents what they offer.
#[must_use]
pub fn asked_when(lang: Lang, probed_at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = now.signed_duration_since(probed_at).num_seconds();
    lang.machines()
        .asked((seconds >= 60).then(|| Ago::of(probed_at, now)))
}

/// An agent's name for people: an added agent's label from `agent.list`,
/// else its name (the page's `agentName` with the list).
#[must_use]
pub fn agent_label(agent: &HarnessRef, agents: &[AgentInfo]) -> String {
    let label = agents
        .iter()
        .filter(|info| info.agent == *agent)
        .find_map(|info| match &info.detail {
            AgentDetail::Added { added, .. } => added.label.as_ref(),
            AgentDetail::Known { .. } => None,
        });
    label.map_or_else(|| agent_name(agent), |label| label.as_str().to_owned())
}
