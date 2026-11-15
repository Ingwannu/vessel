//! Registry rows and search from the node's own platform-resolved catalog.
//!
//! [Decision Log]
//! - Purpose: record an offered command without guessing platform support.
//! - Constraint: agent.add records commands only. A missing package runner
//!   does not prevent recording; Known/Download/Unavailable are not commands.
//! - Choice: exhaustive typed offers and the shared matcher/alphabetical
//!   comparator. Registry identity, not the editable agent name, marks an
//!   already recorded entry, including a different pinned version.
//! - Alternatives: parse the public index again in the app, infer a program
//!   from an archive, or disable Add whenever the runner is missing.
//! - Cost: recording is distinct from installation/readiness. The node still
//!   validates its current catalog version and Admin permission at dispatch.

use vessel_proto::agents::{AgentDetail, AgentInfo, AgentSource, CatalogueEntry, CatalogueRun};
use vessel_proto::{HarnessKind, HarnessRef, HttpUrl};

use crate::{alphabetical, matcher::Query};

pub mod grid;
mod presentation;
pub use presentation::{Activity, Body, Presentation, Read, present};

/// Whether the node found the command's package runner; neither value says
/// that the agent has been installed or signed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runner {
    /// The node found the runner on its search path.
    Found,
    /// Recording is allowed, but running a chat first requires the runner.
    Missing,
}

/// The exact action the node offers for this registry entry on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer<'a> {
    /// This registry id is already recorded, regardless of label or version.
    Recorded(&'a HarnessRef),
    /// Recording this pinned command starts no process or package install.
    Add(Runner),
    /// The registry resolves to a builtin managed by its own agent controls.
    BuiltIn(HarnessKind),
    /// The node does not download archives; a person installs it separately.
    Download(&'a HttpUrl),
    /// No distribution is offered for the node's platform.
    Unavailable,
}

/// Prefer a node-reported registry source over an editable name or an
/// installation guess. Missing runners remain valid record-only offers.
#[must_use]
pub fn offer<'a>(entry: &'a CatalogueEntry, agents: &'a [AgentInfo]) -> Offer<'a> {
    let recorded = agents.iter().find(|agent| match &agent.detail {
        AgentDetail::Known { .. } => false,
        AgentDetail::Added { added } => match &added.source {
            AgentSource::Command => false,
            AgentSource::Registry { id, .. } => *id == entry.id,
        },
    });
    if let Some(agent) = recorded {
        return Offer::Recorded(&agent.agent);
    }
    match &entry.runs {
        CatalogueRun::Known { harness } => Offer::BuiltIn(*harness),
        CatalogueRun::Command { runner_found, .. } => Offer::Add(if *runner_found {
            Runner::Found
        } else {
            Runner::Missing
        }),
        CatalogueRun::Download { archive, .. } => Offer::Download(archive),
        CatalogueRun::Unavailable => Offer::Unavailable,
    }
}

/// Paseo's catalog search over the visible title/id: alphabetical for an
/// empty query, otherwise relevance first and the shared name order for ties.
#[must_use]
pub fn ranked<'a>(entries: &'a [CatalogueEntry], query: &str) -> Vec<&'a CatalogueEntry> {
    let query = Query::new(query);
    if query.is_empty() {
        let mut rows: Vec<_> = entries.iter().collect();
        rows.sort_by(|one, other| alphabetical::compare(&one.name, &other.name));
        return rows;
    }
    let mut rows: Vec<_> = entries
        .iter()
        .filter_map(|entry| {
            query
                .score(&[&entry.name, entry.id.as_str()], None)
                .map(|score| (score, entry))
        })
        .collect();
    rows.sort_by(|(one_score, one), (other_score, other)| {
        one_score
            .cmp(other_score)
            .then_with(|| alphabetical::compare(&one.name, &other.name))
    });
    rows.into_iter().map(|(_, entry)| entry).collect()
}

mod problem;
#[cfg(test)]
mod tests;
mod words;
pub use problem::Problem;

/// A real link the node supplied, never an app-constructed installation URL.
#[derive(Debug, Clone, Copy)]
pub enum Resource<'a> {
    /// The public site given by this registry entry.
    Website(&'a HttpUrl),
    /// A source repository when the entry supplied no site.
    Repository(&'a HttpUrl),
    /// Its explicitly offered platform archive, installed outside this app.
    Archive(&'a HttpUrl),
}
impl<'a> Resource<'a> {
    /// Preserve the original typed HTTP(S) URL at the opener boundary.
    #[must_use]
    pub const fn url(self) -> &'a HttpUrl {
        match self {
            Self::Website(url) | Self::Repository(url) | Self::Archive(url) => url,
        }
    }
}
/// The wire has no installLink field. Name its actual site/source/archive
/// accurately rather than manufacture a vendor-specific documentation URL.
#[must_use]
pub fn resource(entry: &CatalogueEntry) -> Option<Resource<'_>> {
    if let CatalogueRun::Download { archive, .. } = &entry.runs {
        return Some(Resource::Archive(archive));
    }
    entry
        .website
        .as_ref()
        .map(Resource::Website)
        .or_else(|| entry.repository.as_ref().map(Resource::Repository))
}
