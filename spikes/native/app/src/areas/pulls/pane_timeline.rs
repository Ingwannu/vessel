//! The pane's activity read: `pr.timeline` (ADR 0088 part 3) for the
//! pull request the pane read, asked after each whole read (Paseo
//! refetches its timeline when the status changes) and kept while a new
//! one is out (its `keepPreviousData`).
//!
//! [Decision Log]
//! - Purpose: Paseo's Activity lists reviews, comments and threads; the
//!   whole read carries only the reviews' standing.
//! - Constraint: a node before `PULL_REQUEST_PANE_PROTOCOL` has no
//!   timeline; Paseo then shows no activity.
//! - Choice: such a node keeps the standing (the facts dialog's summary),
//!   which says more than "No activity yet"; a failed read keeps the last
//!   timeline, or says nothing new.

use vessel_app_link::workspace::{Ask, PullAsk};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::SessionId;
use vessel_proto::forge::{GetPullTimeline, PULL_REQUEST_PANE_PROTOCOL, PullTimeline};

use crate::shell::Shell;

/// The pane's activity, as read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum TimelineRead {
    /// Not asked: no pull request read yet, or a node without the call.
    #[default]
    Unasked,
    /// The first read is out.
    Loading,
    /// Read; a newer read may be out.
    Ready(Box<PullTimeline>),
    /// The first read failed.
    Failed,
}

impl Shell {
    /// Whether `machine` answers `pr.timeline` and `pr.check`.
    pub(crate) fn pulls_has_pane_reads(&self, machine: &str) -> bool {
        let protocol = self
            .fleet
            .machines
            .iter()
            .find(|known| known.name == machine)
            .and_then(|known| known.protocol);
        matches!((protocol, PULL_REQUEST_PANE_PROTOCOL), (Some(has), Some(needs)) if has >= needs)
    }

    /// Ask the activity of the pull request the pane read.
    pub(super) fn pulls_ask_timeline(&mut self, machine: &str, session: SessionId) {
        if !self.pulls_has_pane_reads(machine) {
            return;
        }
        let Some(facts) = self
            .areas
            .pulls
            .facts
            .as_mut()
            .filter(|facts| facts.subject.machine == machine && facts.subject.session == session)
        else {
            return;
        };
        if !matches!(facts.timeline, TimelineRead::Ready(_)) {
            facts.timeline = TimelineRead::Loading;
        }
        self.ask(
            machine,
            Ask::Pull(PullAsk::Timeline(GetPullTimeline { session })),
        );
    }

    /// The activity arrived.
    pub(super) fn pulls_timeline(
        &mut self,
        machine: &str,
        session: SessionId,
        result: Result<Box<PullTimeline>, Problem>,
    ) {
        let Some(facts) = self
            .areas
            .pulls
            .facts
            .as_mut()
            .filter(|facts| facts.subject.machine == machine && facts.subject.session == session)
        else {
            return;
        };
        facts.timeline = match (result, &facts.timeline) {
            (Ok(timeline), _) => TimelineRead::Ready(timeline),
            (Err(_), TimelineRead::Ready(_)) => return,
            (Err(_), TimelineRead::Unasked | TimelineRead::Loading | TimelineRead::Failed) => {
                TimelineRead::Failed
            }
        };
    }
}
