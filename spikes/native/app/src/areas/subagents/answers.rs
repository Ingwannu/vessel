//! Scope checks precede publication and navigation, including late mutations.
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::Context;
use vessel_app_link::Command;
use vessel_app_view::{
    Card,
    subagents::{ManagedChild, SubagentReply as Reply, Target, TrackProblem, Visibility},
};

impl Shell {
    pub(crate) fn subagents_answer(
        &mut self,
        answer: vessel_app_link::subagents::Answer,
        cx: &mut Context<'_, Self>,
    ) {
        let reply = answer.into_reply();
        match reply {
            Reply::Providers { read, result } => {
                if let Some(track) = self.areas.subagents.track.as_mut() {
                    track.listed(&read, result);
                }
            }
            Reply::Timeline { read, result } => self.subagent_timeline_answer(read, result, cx),
            Reply::LookedUp {
                target,
                ticket,
                result,
            } => self.subagent_lookup_answer(target, ticket, result),
            Reply::Managed {
                host,
                parent,
                ticket,
                result,
            } => self.subagent_managed_answer(host, parent, ticket, result),
            Reply::Detached {
                choice,
                ticket,
                result,
            } => self.subagent_detached_answer(choice, ticket, result),
        }
        self.areas.subagents.reproject(self.words);
        cx.notify();
    }
    fn subagent_timeline_answer(
        &mut self,
        read: vessel_app_view::subagents::ChildRead,
        result: Result<vessel_proto::subagents::ProviderChildPage, TrackProblem>,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(pane) = self.areas.subagents.pane.as_mut()
            && pane.received(&read, result)
            && pane.failure().is_none()
        {
            let source = pane.identity();
            let revision = pane.projection_revision();
            let Some(timeline) = pane.take_projection() else {
                return;
            };
            let mut readonly = crate::areas::conversation::ConversationArea::new(cx);
            readonly.timeline = timeline;
            readonly.logs = vessel_app_view::timeline::Logs::of(readonly.timeline.entries());
            if readonly.folds.is_some() {
                readonly.folds = Some(vessel_app_view::timeline::Folds::of(
                    readonly.timeline.entries(),
                ));
            }
            readonly.rows = vessel_app_view::timeline::Rows::of(
                readonly.timeline.entries().len(),
                readonly.folds.as_ref(),
                &readonly.logs,
                &readonly.open_runs,
            );
            readonly.list.reset(readonly.rows.len());
            self.areas.subagents.readonly = readonly;
            self.areas.subagents.readonly_source = Some(source);
            self.areas.subagents.readonly_revision = revision;
        }
    }
    fn subagent_managed_answer(
        &mut self,
        host: vessel_app_view::subagents::Host,
        parent: vessel_proto::SessionRef,
        ticket: vessel_app_view::devices::Ticket,
        result: Result<Vec<vessel_proto::SessionInfo>, TrackProblem>,
    ) {
        let area = &mut self.areas.subagents;
        let pending = area
            .hosts
            .iter()
            .position(|r| r.host == host && r.parent == parent && r.ticket == ticket);
        if let Some(index) = pending
            && area
                .track
                .as_ref()
                .is_some_and(|track| track.target().address == parent)
        {
            area.hosts.remove(index);
            match result {
                Ok(infos) => {
                    if area
                        .uncertain
                        .as_ref()
                        .is_some_and(|target| target.address.node == host.node)
                    {
                        area.uncertain = None;
                    }
                    let rows = infos
                        .into_iter()
                        .map(|info| ManagedChild {
                            target: Target::new(host.context.clone(), info.id),
                            visibility: if info.marks.archived_at.is_some() {
                                Visibility::Archived
                            } else {
                                Visibility::Active
                            },
                            info: Box::new(info),
                            finished: None,
                        })
                        .collect();
                    if let Some(track) = area.track.as_mut()
                        && let Err(problem) = track.managed_host(&host, rows)
                    {
                        area.failure = Some(problem);
                    }
                }
                Err(problem) => {
                    area.failure = Some(problem);
                    area.partial = true;
                }
            }
        }
    }
    fn subagent_detached_answer(
        &mut self,
        choice: vessel_app_view::subagents::DetachChoice,
        ticket: vessel_app_view::devices::Ticket,
        result: Result<vessel_proto::subagents::DetachedSession, TrackProblem>,
    ) {
        let matching = self.areas.subagents.detaching.as_ref() == Some(&(choice.clone(), ticket));
        if matching {
            self.areas.subagents.detaching = None;
            let current = self
                .areas
                .subagents
                .track
                .as_ref()
                .is_some_and(|track| track.target().address == choice.parent);
            match result {
                Ok(detached) => {
                    self.command(Command::Reload);
                    if current
                        && self.areas.subagents.open
                        && self.route == Route::Session
                        && self.subagent_target_current(&choice.child)
                    {
                        let card =
                            Card::from_session(choice.child.machine.as_str(), &detached.session);
                        self.areas.subagents.pending_open =
                            Some(crate::areas::session::Open::of(&card));
                        self.areas.subagents.open = false;
                    }
                    self.subagents_refresh();
                }
                Err(problem) => {
                    if problem == TrackProblem::Uncertain {
                        self.areas.subagents.uncertain = Some(choice.child);
                    }
                    if current {
                        self.areas.subagents.failure = Some(problem);
                    }
                }
            }
        }
    }
    fn subagent_lookup_answer(
        &mut self,
        target: Target,
        ticket: vessel_app_view::devices::Ticket,
        result: Result<Box<vessel_proto::SessionInfo>, TrackProblem>,
    ) {
        if self.areas.subagents.navigate.as_ref() == Some(&(target.clone(), ticket)) {
            self.areas.subagents.navigate = None;
            match result {
                Ok(info)
                    if self.route == Route::Session && self.subagent_target_current(&target) =>
                {
                    let card = Card::from_session(target.machine.as_str(), &info);
                    self.areas.subagents.pending_open =
                        Some(crate::areas::session::Open::of(&card));
                    self.areas.subagents.open = false;
                }
                Ok(_) => {}
                Err(problem) => self.areas.subagents.failure = Some(problem),
            }
        } else if self.areas.subagents.lookup.as_ref() == Some(&(target.clone(), ticket)) {
            self.areas.subagents.lookup = None;
            if !self.subagent_target_current(&target) {
                self.areas.subagents.failure = Some(TrackProblem::IdentityChanged);
                return;
            }
            match result {
                Ok(info) => self.areas.subagents.info = Some(info),
                Err(problem) => self.areas.subagents.failure = Some(problem),
            }
        }
    }
    pub(super) fn subagent_target_current(&self, target: &Target) -> bool {
        self.fleet
            .machines
            .iter()
            .find(|machine| machine.name == target.machine.as_str())
            .is_some_and(|machine| machine.node == Some(target.address.node))
    }
}
