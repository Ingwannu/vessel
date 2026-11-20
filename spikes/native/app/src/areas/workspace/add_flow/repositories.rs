//! Clone from GitHub's search (TRACKER 440; `forge.repos`, ADR 0088): the
//! page asks its machine after the field's pause, as the folders are
//! asked, and keeps the answer with the machine and the text it answers
//! (split from `add_flow.rs` for the file budget).
//!
//! [Decision Log]
//! - Purpose: Paseo's GitHub page lists the account's repositories (an
//!   empty field) and a search's (typed text), 30 at a time.
//! - Choice: one answer kept, counted only while its machine and text are
//!   the page's (the page's `repositories.ts`); a failed call keeps no
//!   rows and says Paseo's generic words, never the node's error text.

use vessel_app_link::workspace::{Ask, ProjectAsk};
use vessel_app_view::workspace::add_flow::model::PageId;
use vessel_app_view::workspace::add_flow::repositories::{REPOSITORY_ROWS, Searched, searched};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::forge::{RepoLimit, RepoQuery, RepositoryList, SearchRepositories};

use super::Open;
use crate::shell::Shell;

/// One `forge.repos` answer and what it was asked for.
#[derive(Debug, Clone)]
pub(super) struct Repositories {
    machine: String,
    query: String,
    /// The list; `None` when the call failed.
    found: Option<RepositoryList>,
}

/// The machine and query the page searches with, when it searches: the
/// GitHub page, a machine that has `forge.repos`, a field that can be a
/// query.
fn asks(open: &Open) -> Option<(String, RepoQuery)> {
    let page = open.flow.page();
    if page.kind.id() != PageId::GithubSearch {
        return None;
    }
    let machine = open.flow.machine().filter(|machine| machine.can_search_github)?;
    let query = page.field.parse().ok()?;
    Some((machine.name.clone(), query))
}

/// What the page's search shows for the field as it is now: the answer
/// for this machine and text, else loading while it searches.
pub(super) fn shown(open: &Open) -> Searched {
    let Some((machine, query)) = asks(open) else {
        return Searched::default();
    };
    match &open.repositories {
        Some(answer) if answer.machine == machine && answer.query == query.as_str() => {
            searched(answer.found.as_ref())
        }
        Some(_) | None => Searched {
            loading: true,
            ..Searched::default()
        },
    }
}

impl Shell {
    /// Ask the page's machine for repositories matching the field, on the
    /// GitHub page of a machine that answers.
    pub(super) fn add_flow_ask_repositories(&mut self) {
        let Some(open) = &self.areas.workspace.add_flow.open else {
            return;
        };
        let Some((machine, query)) = asks(open) else {
            return;
        };
        let Ok(limit) = RepoLimit::new(REPOSITORY_ROWS) else {
            return;
        };
        self.ask(
            &machine,
            Ask::Project(ProjectAsk::SearchRepositories(SearchRepositories {
                query,
                limit,
            })),
        );
    }

    /// Repositories arrived for `query` on `machine`.
    pub(in crate::areas::workspace) fn add_flow_repositories(
        &mut self,
        machine: &str,
        query: &str,
        result: Result<RepositoryList, Problem>,
    ) {
        let Some(open) = &mut self.areas.workspace.add_flow.open else {
            return;
        };
        open.repositories = Some(Repositories {
            machine: machine.to_owned(),
            query: query.to_owned(),
            found: result.ok(),
        });
    }
}
