//! Read-only Sync, after Paseo's `screens/settings/host-page.tsx`:
//! section/card/row facts, short status badges and adjacent inline errors.
//!
//! [Decision Log]
//! - Purpose: inspect the node's shared names and last member reports.
//! - Constraints: the wire offers no join/leave/force-sync action or a
//!   member's home identity. Raw transports never arrive in this area.
//! - Alternatives: guessing a home from list order; drawing dead actions.
//! - Choice: render the typed partial snapshot, with local CLI guidance
//!   using the kit's copy control. Scope/tickets are the tested view rules.
//! - Cost: a member's actual applied state is checked with local status;
//!   this page describes only the reports the node exposes.

mod commands;
mod frame;
mod members;
mod shared;

use crate::{routes::Route, shell::Shell};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{Tokens, Tooltips};
use vessel_app_view::devices::{MachineKey, Target};
use vessel_app_view::sync::{Page, Problem, Reading};
use vessel_app_view::{Reach, settings::Section};
use vessel_proto::DeviceId;

/// Only UI handles and the viewer's captured offset; no raw node values.
pub(crate) struct SyncState {
    page: Page,
    offset: chrono::FixedOffset,
    tips: Tooltips,
    places: HashMap<DeviceId, vessel_app_kit::tooltip::Place>,
    /// Each shared file's path tooltip, kept across frames by its path.
    paths: std::cell::RefCell<HashMap<String, vessel_app_kit::tooltip::Place>>,
}
impl SyncState {
    pub(super) fn new() -> Self {
        Self {
            page: Page::default(),
            offset: *chrono::Local::now().offset(),
            tips: Tooltips::new(),
            places: HashMap::new(),
            paths: std::cell::RefCell::default(),
        }
    }
}
impl Shell {
    /// Scope is reconciled before drawing, including navigation elsewhere.
    pub(crate) fn settle_sync(&mut self, window: &Window) {
        let target = if self.route != Route::Settings
            || self.areas.settings.section != Section::Sync
            || vessel_app_kit::breakpoint::phone(window) && !self.areas.settings.pushed
        {
            Target::Hidden
        } else if !self.loaded {
            Target::Loading
        } else {
            match super::choices::chosen_machine(self) {
                None => Target::NoMachines,
                Some(machine) => {
                    let key = MachineKey::from_book(&machine.name);
                    match machine.reach {
                        Reach::Online => Target::Online(key),
                        Reach::Unreachable(_) => Target::Offline(key),
                    }
                }
            }
        };
        let state = &mut self.areas.settings.sync;
        if state.page.target != target {
            state.tips = Tooltips::new();
            state.places.clear();
        }
        state.page.target(target);
        if matches!(state.page.target, Target::Online(_))
            && matches!(state.page.reading, Reading::Empty)
        {
            self.sync_read();
        }
    }

    fn sync_read(&mut self) {
        let state = &mut self.areas.settings.sync;
        let Target::Online(machine) = &state.page.target else {
            return;
        };
        if matches!(state.page.reading, Reading::Loading(_)) {
            return;
        }
        let ticket = match state.page.tickets.next_ticket() {
            Ok(ticket) => ticket,
            Err(_exhausted) => {
                state.page.reading = Reading::Failed(Problem::Exhausted);
                return;
            }
        };
        let machine = machine.clone();
        state.page.reading = Reading::Loading(ticket);
        if let Err(why) = self.sync_send(machine, ticket) {
            self.areas
                .settings
                .sync
                .page
                .reading
                .answer(ticket, Err(why));
        }
    }

    pub(crate) fn sync_answer(&mut self, answer: vessel_app_link::sync::Answer) {
        let state = &mut self.areas.settings.sync;
        state.page.reading.answer(answer.ticket, answer.result);
        state.places.clear();
        state.tips = Tooltips::new();
        if let Reading::Ready(snapshot) = &state.page.reading
            && let Ok(members) = &snapshot.members
        {
            for member in members {
                let _previous = state.places.insert(member.device, Rc::new(Cell::new(None)));
            }
        }
    }
}

pub(super) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    frame::render(shell, tokens, compact, window, cx)
}
