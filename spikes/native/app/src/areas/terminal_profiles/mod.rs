//! A machine's terminal profiles in the app (ADR 0117): one list per
//! machine, read for New tab's profile rows and for Settings › Terminals,
//! and changed there.
//!
//! [Decision Log]
//! - Purpose: Paseo's daemon config holds `terminalProfiles`; its New tab
//!   lists them under "Terminal profiles" and its host page edits them
//!   (`screens/settings/host-page.tsx` 1311-1665). Vessel's node keeps the
//!   list in its settings (protocol 45).
//! - Before: New tab drew the shipped four, disabled; Settings › Terminals
//!   was not drawn.
//! - Considered: each surface asking on its own (two copies that can
//!   disagree after a save); reading on every frame (a call a frame).
//! - Choice: one `view::terminal_profiles::Listing` a machine, shared by
//!   both surfaces, so a save in Settings shows in New tab at once. It is
//!   read when a workspace on that machine is shown, when Settings ›
//!   Terminals is shown for it, and again when the window comes back to
//!   the front (Paseo's config query refetches on focus), so a change made
//!   elsewhere shows on return.
//! - Cost: rows read from another device stay as read until one of those
//!   moments; nothing pushes a change.

mod dialog;
pub(crate) mod menu;
pub(crate) mod page;

use std::collections::HashMap;

use vessel_app_kit::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::terminal_profiles::{Answer, Ask};
use vessel_app_view::Reach;
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::layout::LayoutKey;
use vessel_app_view::settings::Section;
use vessel_app_view::terminal_profiles::{Listing, Problem};
use vessel_proto::terminal_profiles::{Id, Profiles};

use crate::routes::Route;
use crate::shell::Shell;

/// Every machine's list, and Settings › Terminals' own state.
pub(crate) struct State {
    listings: HashMap<MachineKey, Listing>,
    tickets: Tickets,
    /// The machine Settings › Terminals read since it was shown.
    page: Option<MachineKey>,
    /// A move or removal the machine did not take, said over the list.
    refused: Option<Problem>,
    /// The edit dialog, while open.
    edit: Option<dialog::Edit>,
    edit_dialog: DialogState,
    /// The row the remove confirmation asks about.
    removing: Option<Id>,
    remove_dialog: DialogState,
    _activation: gpui::Subscription,
}

impl State {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let activation = cx.observe_window_activation(window, |shell, window, _cx| {
            if window.is_window_active() {
                shell.terminal_profiles_refresh();
            }
        });
        Self {
            listings: HashMap::new(),
            tickets: Tickets::default(),
            page: None,
            refused: None,
            edit: None,
            edit_dialog: DialogState::new(cx),
            removing: None,
            remove_dialog: DialogState::new(cx),
            _activation: activation,
        }
    }
}

/// The machine of the workspace shown, by its book name.
fn workspace_machine(shell: &Shell) -> Option<String> {
    match crate::areas::layout::current_key(shell).ok()? {
        LayoutKey::Session { machine, .. } => Some(machine.as_str().to_owned()),
        LayoutKey::Workspace { .. } => None,
    }
}

impl Shell {
    /// The shown workspace machine's profiles, once read (New tab's rows):
    /// none while unread, below protocol 45, or when the read failed, so
    /// no row is shown that the node would not run.
    pub(crate) fn terminal_profiles_shown(&self) -> Option<&Profiles> {
        let machine = workspace_machine(self)?;
        self.areas
            .terminal_profiles
            .listings
            .get(&MachineKey::from_book(&machine))?
            .profiles()
    }

    /// Ask `machine` for its list; a save waiting brings it anyway.
    fn terminal_profiles_read(&mut self, machine: MachineKey) {
        let state = &mut self.areas.terminal_profiles;
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        let listing = state.listings.entry(machine.clone()).or_default();
        if listing.saving() {
            return;
        }
        listing.reading(ticket);
        self.command(Command::TerminalProfiles(Ask {
            machine,
            ticket,
            save: None,
        }));
    }

    /// With the terminal tabs' poll: the shown workspace's machine, while
    /// it is connected, is read once, and again when the last read found
    /// it offline.
    pub(crate) fn terminal_profiles_ensure(&mut self, machine: &str) {
        let key = MachineKey::from_book(machine);
        let unread = self
            .areas
            .terminal_profiles
            .listings
            .get(&key)
            .is_none_or(|listing| {
                !listing.waiting() && listing.problem() == Some(Problem::Offline)
            });
        if unread {
            self.terminal_profiles_read(key);
        }
    }

    /// The window came to the front: every connected machine's list is
    /// read again.
    fn terminal_profiles_refresh(&mut self) {
        let online: Vec<MachineKey> = self
            .fleet
            .online()
            .map(|machine| MachineKey::from_book(&machine.name))
            .filter(|key| self.areas.terminal_profiles.listings.contains_key(key))
            .collect();
        for machine in online {
            self.terminal_profiles_read(machine);
        }
    }

    /// Before drawing: Settings › Terminals reads its machine's list each
    /// time it is shown for it (Paseo's query on mount); leaving it closes
    /// what it opened, unless a save waits.
    pub(crate) fn settle_terminal_profiles(&mut self, window: &Window) {
        let shown = self.route == Route::Settings
            && self.areas.settings.section == Section::Terminals
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let online = crate::areas::settings::chosen_machine(self)
            .filter(|machine| shown && machine.reach == Reach::Online)
            .map(|machine| MachineKey::from_book(&machine.name));
        let state = &mut self.areas.terminal_profiles;
        let Some(machine) = online else {
            if !shown {
                state.page = None;
                state.refused = None;
                state.removing = None;
                if state.edit.as_ref().is_some_and(|edit| !edit.saving()) {
                    state.edit = None;
                }
            }
            return;
        };
        if state.page.as_ref() == Some(&machine) {
            return;
        }
        state.page = Some(machine.clone());
        state.refused = None;
        self.terminal_profiles_read(machine);
    }

    /// Keep `list` as `machine`'s whole list. Whether it was sent: not
    /// while another save waits.
    fn terminal_profiles_save(&mut self, machine: MachineKey, list: Profiles) -> bool {
        let state = &mut self.areas.terminal_profiles;
        let Ok(ticket) = state.tickets.next_ticket() else {
            return false;
        };
        let listing = state.listings.entry(machine.clone()).or_default();
        if !listing.saving_with(ticket) {
            return false;
        }
        state.refused = None;
        self.command(Command::TerminalProfiles(Ask {
            machine,
            ticket,
            save: Some(list),
        }));
        true
    }

    /// The machine answered. A save the edit dialog sent closes it, or is
    /// said inside it; a move's or a removal's refusal is said over the
    /// list.
    pub(crate) fn terminal_profiles_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let Answer {
            machine,
            ticket,
            result,
        } = answer;
        let state = &mut self.areas.terminal_profiles;
        let Some(saved) = state
            .listings
            .get_mut(&machine)
            .and_then(|listing| listing.answered(ticket, result))
        else {
            return;
        };
        let from_dialog = state
            .edit
            .as_ref()
            .is_some_and(|edit| edit.saving() && edit.machine() == &machine);
        match saved {
            Ok(()) if from_dialog => state.edit = None,
            Ok(()) => {}
            Err(problem) if from_dialog => {
                if let Some(edit) = state.edit.as_mut() {
                    edit.refused(problem, cx);
                }
            }
            Err(problem) => state.refused = Some(problem),
        }
    }
}

#[cfg(test)]
impl Shell {
    /// `machine`'s list as if read from its node.
    pub(crate) fn terminal_profiles_read_as(&mut self, machine: &str, list: Profiles) {
        let state = &mut self.areas.terminal_profiles;
        let Ok(ticket) = state.tickets.next_ticket() else {
            return;
        };
        let listing = state
            .listings
            .entry(MachineKey::from_book(machine))
            .or_default();
        listing.reading(ticket);
        let _read = listing.answered(ticket, Ok(list));
    }
}

#[cfg(test)]
mod tests;
