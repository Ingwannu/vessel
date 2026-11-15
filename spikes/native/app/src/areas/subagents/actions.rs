//! Every action captures a full address before entering an asynchronous boundary.
use super::HostRead;
use crate::{areas::session::Open, shell::Shell};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::subagents::{
    ChildPane, ChildQuery, Host, SubagentAsk as Ask, Target, Track, TrackProblem,
};
use vessel_proto::{SessionRef, subagents::ProviderChildId};

impl Shell {
    pub(crate) fn subagents_opened(&mut self, open: &Open) {
        if !open.structured {
            self.areas.subagents.track = None;
            return;
        }
        let Some(context) = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == open.machine)
            .and_then(vessel_app_view::Machine::connection_context)
        else {
            self.areas.subagents.track = None;
            return;
        };
        let target = Target::new(context, open.session);
        if self
            .areas
            .subagents
            .track
            .as_ref()
            .is_some_and(|track| track.target() == &target)
        {
            return;
        }
        let area = &mut self.areas.subagents;
        area.track = Some(Track::new(target));
        area.info = None;
        area.lookup = None;
        area.navigate = None;
        area.hosts.clear();
        area.rows.clear();
        area.list.reset(0);
        area.pane = None;
        area.open = false;
        area.pending_open = None;
        area.readonly_source = None;
        area.readonly_revision = None;
        area.confirm = None;
        area.failure = None;
        area.partial = true;
        area.reproject(self.words);
        self.subagents_refresh();
    }
    pub(crate) fn subagents_fleet_changed(&mut self) {
        if let Some(open) = self.areas.session.open.clone() {
            self.subagents_opened(&open);
        }
        let area = &mut self.areas.subagents;
        let relation = area.track.as_ref().and_then(|track| {
            self.fleet
                .machines
                .iter()
                .find(|machine| machine.node == Some(track.target().address.node))
                .and_then(|machine| {
                    machine
                        .sessions
                        .iter()
                        .find(|card| card.session == track.target().address.session)
                })
                .map(|card| card.detail.relation)
        });
        if let Some(vessel_app_view::subagents::Relation::Known(current)) = relation
            && let Some(info) = &area.info
            && info
                .parentage()
                .ok()
                .and_then(vessel_proto::subagents::Parentage::parent)
                != current
        {
            // Hide stale parent navigation until a fresh authoritative lookup settles.
            area.info = None;
            if area.lookup.is_none() {
                self.subagents_refresh();
            }
        }
    }
    pub(super) fn open_subagents(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.subagents;
        area.open = true;
        area.prior = window.focused(cx);
        area.dialog.focus(window, cx);
        self.subagents_refresh();
        cx.notify();
    }
    pub(super) fn close_subagents(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.subagents;
        area.open = false;
        area.confirm = None;
        if let Some(prior) = area.prior.take() {
            prior.focus(window, cx);
        }
        cx.notify();
    }
    pub(super) fn subagents_refresh(&mut self) {
        let Some(target) = self
            .areas
            .subagents
            .track
            .as_ref()
            .map(|track| track.target().clone())
        else {
            return;
        };
        self.areas.subagents.failure = None;
        if let Ok(ticket) = self.areas.subagents.tickets.next_ticket() {
            self.areas.subagents.lookup = Some((target.clone(), ticket));
            self.command(Command::Subagents(Ask::Lookup {
                target: target.clone(),
                ticket,
            }));
        }
        let read = {
            let area = &mut self.areas.subagents;
            area.track
                .as_mut()
                .and_then(|track| track.refresh(&mut area.tickets).ok())
        };
        if let Some(read) = read {
            self.command(Command::Subagents(Ask::Providers(read)));
        }
        let hosts = self
            .fleet
            .online()
            .filter_map(vessel_app_view::Machine::connection_context)
            .map(Host::new)
            .collect::<Vec<_>>();
        self.areas.subagents.hosts.clear();
        for host in hosts {
            let Ok(ticket) = self.areas.subagents.tickets.next_ticket() else {
                self.areas.subagents.failure = Some(TrackProblem::Exhausted);
                break;
            };
            self.areas.subagents.hosts.push(HostRead {
                host: host.clone(),
                parent: target.address,
                ticket,
            });
            self.command(Command::Subagents(Ask::Managed {
                host,
                parent: target.address,
                ticket,
            }));
        }
    }
    pub(super) fn subagent_provider(&mut self, child: &ProviderChildId) {
        let Some(target) = self
            .areas
            .subagents
            .track
            .as_ref()
            .map(|track| track.target().clone())
        else {
            return;
        };
        if self
            .areas
            .subagents
            .track
            .as_ref()
            .and_then(|track| track.provider(child))
            .is_none()
        {
            return;
        }
        let mut pane = ChildPane::new(ChildQuery {
            target,
            child: child.clone(),
        });
        let read = pane.read(None, &mut self.areas.subagents.tickets);
        self.areas.subagents.pane = Some(pane);
        self.areas.subagents.readonly_source = None;
        self.areas.subagents.readonly_revision = None;
        self.areas.subagents.reproject(self.words);
        if let Ok(read) = read {
            self.command(Command::Subagents(Ask::Timeline(read)));
        }
    }
    pub(super) fn subagent_provider_back(&mut self) {
        let parent = self.areas.subagents.pane.as_ref().and_then(|pane| {
            self.areas
                .subagents
                .track
                .as_ref()
                .and_then(|track| track.provider(&pane.query().child))
                .and_then(|row| row.parent.as_ref())
                .cloned()
        });
        if let Some(parent) = parent {
            self.subagent_provider(&parent);
        } else {
            self.areas.subagents.pane = None;
            self.areas.subagents.readonly_source = None;
            self.areas.subagents.readonly_revision = None;
            self.areas.subagents.reproject(self.words);
        }
    }
    pub(super) fn subagent_older(&mut self) {
        let Some(pane) = self.areas.subagents.pane.as_mut() else {
            return;
        };
        let Some(before) = pane.before() else {
            return;
        };
        if let Ok(read) = pane.read(Some(before), &mut self.areas.subagents.tickets) {
            self.command(Command::Subagents(Ask::Timeline(read)));
        }
    }
    pub(super) fn subagent_hide(&mut self, child: &ProviderChildId) {
        if let Some(track) = self.areas.subagents.track.as_mut() {
            let _hidden = track.hide_provider(child);
        }
        self.areas.subagents.reproject(self.words);
    }
    pub(super) fn subagents_hide_finished(&mut self) {
        if let Some(track) = self.areas.subagents.track.as_mut() {
            track.hide_finished_providers();
        }
        self.areas.subagents.reproject(self.words);
    }
    pub(super) fn subagent_managed(
        &mut self,
        child: SessionRef,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let row = self
            .areas
            .subagents
            .track
            .as_ref()
            .and_then(|track| {
                track
                    .managed()
                    .iter()
                    .find(|row| row.target.address == child)
            })
            .cloned();
        if let Some(row) = row {
            self.subagent_navigate(row.target, window, cx);
        }
    }
    pub(super) fn subagent_navigate(
        &mut self,
        target: Target,
        _window: &mut Window,
        _cx: &mut Context<'_, Self>,
    ) {
        let Ok(ticket) = self.areas.subagents.tickets.next_ticket() else {
            return;
        };
        self.areas.subagents.navigate = Some((target.clone(), ticket));
        self.command(Command::Subagents(Ask::Lookup { target, ticket }));
    }
    pub(super) fn subagent_parent(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let parent = self
            .areas
            .subagents
            .info
            .as_ref()
            .and_then(|info| info.parentage().ok())
            .and_then(vessel_proto::subagents::Parentage::parent);
        let Some(parent) = parent else {
            return;
        };
        let Some(machine) = self.fleet.online().find(|m| m.node == Some(parent.node)) else {
            self.areas.subagents.failure = Some(TrackProblem::Offline);
            return;
        };
        let Some(context) = machine.connection_context() else {
            self.areas.subagents.failure = Some(TrackProblem::IdentityChanged);
            return;
        };
        self.subagent_navigate(Target::new(context, parent.session), window, cx);
    }
    pub(super) fn subagent_confirm(
        &mut self,
        child: SessionRef,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.subagents.detaching.is_some() || self.areas.subagents.uncertain.is_some() {
            return;
        }
        self.areas.subagents.confirm = self
            .areas
            .subagents
            .track
            .as_ref()
            .and_then(|track| track.detach_choice(child));
        self.areas.subagents.confirm_dialog.focus(window, cx);
        cx.notify();
    }
    pub(super) fn subagent_cancel_detach(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.subagents.confirm = None;
        self.areas.subagents.dialog.focus(window, cx);
        cx.notify();
    }
    pub(super) fn subagent_detach(&mut self) {
        if self.areas.subagents.detaching.is_some() {
            return;
        }
        let Some(choice) = self.areas.subagents.confirm.take() else {
            return;
        };
        let Ok(ticket) = self.areas.subagents.tickets.next_ticket() else {
            return;
        };
        self.areas.subagents.detaching = Some((choice.clone(), ticket));
        self.command(Command::Subagents(Ask::Detach { choice, ticket }));
    }
    pub(super) fn subagents_open_pending(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(open) = self.areas.subagents.pending_open.take()
            && self.route == crate::routes::Route::Session
        {
            self.open_session(open, window, cx);
        }
    }
    pub(crate) fn subagents_records(
        &mut self,
        session: &vessel_proto::SessionId,
        records: &[vessel_proto::EventRecord],
    ) {
        let matches = self
            .areas
            .subagents
            .track
            .as_ref()
            .is_some_and(|track| track.target().address.session == *session);
        if matches
            && records.iter().any(|r| {
                matches!(
                    r.event,
                    vessel_proto::SessionEvent::Agent {
                        event: vessel_proto::AgentEvent::ProviderChild(_)
                    }
                )
            })
        {
            if let Some(pane) = self.areas.subagents.pane.as_mut()
                && !pane.loading()
            {
                let after = pane.after();
                if let Ok(read) = pane.read(Some(after), &mut self.areas.subagents.tickets) {
                    self.command(Command::Subagents(Ask::Timeline(read)));
                }
            }
            let read = {
                let area = &mut self.areas.subagents;
                area.track
                    .as_mut()
                    .filter(|track| {
                        !matches!(
                            track.load(),
                            vessel_app_view::subagents::TrackLoad::Loading(_)
                        )
                    })
                    .and_then(|track| track.refresh(&mut area.tickets).ok())
            };
            if let Some(read) = read {
                self.command(Command::Subagents(Ask::Providers(read)));
            }
        }
    }
}
