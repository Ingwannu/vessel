//! The shell's exhaustive link-event dispatch, with no drawing or transport.
//!
//! [Decision Log]
//! - Purpose: keep the window root below its file budget as areas register.
//! - Constraint: one dispatcher, unchanged event order/body and no wider API.
//! - Choice: a child of shell; pub(super) has the original shell-only boundary.
//! - Why and cost: ordinary module tooling sees this file, with no include! or
//!   duplicated dispatcher. The existing exhaustive match is deliberately kept
//!   intact in this move rather than mixing an event-policy refactor into it.
use super::{Context, Event, Route, Shell};

impl Shell {
    pub(super) fn on_event(&mut self, event: Event, cx: &mut Context<'_, Self>) {
        match event {
            Event::Sessions(answer) => self.sessions_navigation_answer(answer, cx),
            Event::SessionLifecycle(answer) => self.sessions_lifecycle_answer(answer, cx),
            Event::History(reply) => self.history_answer(reply, cx),
            Event::SessionMarked(answer) => self.session_mark_answer(answer, cx),
            Event::Devices(answer) => self.devices_answer(answer, cx),
            Event::DeviceRetired(context) => {
                self.history_retired(&context, cx);
                if !self.devices_retired(&context) {
                    return;
                }
            }
            Event::Machines(answer) => self.machines_answer(answer, cx),
            Event::Sync(answer) => self.sync_answer(answer),
            Event::NodeSettings(answer) => self.node_settings_answer(answer),
            Event::Fleet(fleet) => {
                // The link reads the fleet every 2 s; a read that changed
                // nothing draws nothing (an idle window repainted every
                // 2 s, measured by the coordinator on 0.0.0.0.13).
                if self.loaded && fleet.machines == self.fleet.machines {
                    self.fleet.read_at = fleet.read_at;
                    if self.route == Route::Settings {
                        self.agent_load(false, cx);
                    }
                    return;
                }
                let earlier = std::mem::replace(&mut self.fleet, fleet);
                self.history_fleet(&earlier);
                self.devices_fleet();
                self.offers_fleet(&earlier);
                if !self.loaded {
                    self.loaded = true;
                    // With no machine there is nothing to type to: pairing
                    // is the first thing to do.
                    if self.fleet.machines.is_empty() {
                        self.route_to(Route::Pair);
                        self.pairing_opened(cx);
                    }
                }
                self.settle_draft();
                self.retitle();
                self.workspace_fleet();
                self.sessions_fleet_changed();
                self.refresh_fork_headers();
                if self.route == Route::Settings {
                    self.agent_load(false, cx);
                }
            }
            Event::Agents {
                machine,
                request,
                answer,
            } => self.agents_answer(&machine, request, answer, cx),
            Event::Records { session, records } => {
                if self.link_records(&session, &records, cx) == crate::areas::Redraw::GridAlone {
                    return;
                }
            }
            Event::Started(card) => self.started_with_files(&card, cx),
            Event::Sent { .. } => {}
            Event::Paired { node, via, address } => self.paired(&node, via, address, cx),
            Event::PairFailed(why) => self.pair_failed(why),
            Event::BookUnreadable(fault) => self.book_unreadable(&fault),
            Event::SessionLost { session, why } => self.session_lost(session, why),
            Event::Acted(answer) => {
                if let Some(answer) = self
                    .board_acted(answer, cx)
                    .and_then(|answer| self.option_acted(answer, cx))
                {
                    self.acted(answer, cx);
                }
                self.refresh_fork_headers();
            }
            Event::Resumed { session } => self.resumed(session, cx),
            Event::ResumeFailed { session, why } => self.resume_failed(session, why),
            Event::Options {
                machine,
                harness,
                outcome,
            } => self.options_arrived(machine, harness, outcome),
            Event::OptionsKept { machine, lists } => self.offers_kept(&machine, lists),
            Event::Upload { session, key, step } => self.composer_upload(session, key, step, cx),
            Event::Failed(why) => {
                self.start_failed();
                self.notice = Some(why);
            }
            Event::Workspace { machine, answer } => {
                // An answer that changed nothing drawn asks for no frame
                // (an idle git panel refreshes every five seconds).
                if !self.workspace_answer(&machine, answer, cx) {
                    return;
                }
            }
            Event::Searched(answer) => self.message_searched(answer, cx),
        }
        cx.notify();
    }
}

#[cfg(test)]
mod import_retire_tests;
