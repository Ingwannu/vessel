//! The pull request pane's reads (ADR 0088): `pr.timeline` and `pr.check`,
//! asked of the forge when the pane asks, for the pull request the
//! watcher holds for the session.
//!
//! [Decision Log]
//! - Purpose: Paseo's pane shows a pull request's activity and a failed
//!   check's logs; the watcher keeps only the facts a card and the board
//!   need.
//! - Constraint: a pull request is reached through its session (ADR
//!   0029): the repository and number are the watcher's, and a check is
//!   one the watcher's last read named, never one a client makes up.
//! - Choice: read the forge on each call (a person's priority), not kept;
//!   the pane asks once when it opens and on refresh, as Paseo's queries.

use vessel_forge::ForgeError;
use vessel_proto::forge::{
    CheckDetails, ForgeAccess, ForgeHost, PullRequestView, PullTimeline, RunId,
};

use super::Forges;
use super::hosts::denied;

impl Forges {
    /// `pr.timeline` for the pull request `view` holds.
    pub(crate) async fn timeline(&self, view: PullRequestView) -> PullTimeline {
        let not_read = |access| PullTimeline {
            access,
            number: None,
            items: Vec::new(),
            truncated: false,
        };
        let (ForgeAccess::Ready { .. }, Some(pull)) = (&view.access, &view.pull_request) else {
            return not_read(view.access);
        };
        let Some(forge) = self.hosts.forge() else {
            return not_read(ForgeAccess::NotSupported {
                host: pull.summary.repo.host.clone(),
            });
        };
        let repo = &pull.summary.repo;
        match forge.timeline(repo, pull.summary.number).await {
            Ok(timeline) => PullTimeline {
                access: view.access.clone(),
                number: Some(pull.summary.number),
                items: timeline.items,
                truncated: timeline.truncated,
            },
            Err(error) => not_read(self.refused(&repo.host, &error)),
        }
    }

    /// `pr.check` for `run`, when it is one of the checks `view` holds.
    pub(crate) async fn check(&self, view: PullRequestView, run: RunId) -> CheckDetails {
        let (ForgeAccess::Ready { .. }, Some(pull)) = (&view.access, &view.pull_request) else {
            return CheckDetails {
                access: view.access,
                report: None,
            };
        };
        let Some(known) = pull
            .checks
            .runs
            .iter()
            .chain(&pull.checks.failing)
            .find(|check| check.run == Some(run))
        else {
            return CheckDetails {
                access: view.access.clone(),
                report: None,
            };
        };
        let Some(forge) = self.hosts.forge() else {
            return CheckDetails {
                access: ForgeAccess::NotSupported {
                    host: pull.summary.repo.host.clone(),
                },
                report: None,
            };
        };
        let repo = &pull.summary.repo;
        match forge.check(repo, run, known.workflow_run).await {
            Ok(report) => CheckDetails {
                access: view.access.clone(),
                report: Some(Box::new(report)),
            },
            Err(ForgeError::NotFound) => CheckDetails {
                access: view.access.clone(),
                report: None,
            },
            Err(error) => CheckDetails {
                access: self.refused(&repo.host, &error),
                report: None,
            },
        }
    }

    fn refused(&self, host: &ForgeHost, error: &ForgeError) -> ForgeAccess {
        denied(host, error, self.hosts.login(host))
    }
}
