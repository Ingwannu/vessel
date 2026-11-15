//! Settings > Devices, on the existing settings frame and kit only.
//! Pure state and credential policy live in `view::devices`; this area
//! holds field/focus handles and wires their actions to `link::devices`.

mod invitation;
mod revoke;
mod row;
mod rows;

use std::{cell::Cell, collections::HashMap, rc::Rc};
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{DialogState, TextField, Tooltips};
use vessel_app_link::devices::{Answer, Ask};
use vessel_app_view::devices::{Listing, MachineKey, Page, Target};
use vessel_app_view::{Reach, settings::Section};
use vessel_proto::DeviceId;

use crate::{routes::Route, shell::Shell};

pub(crate) struct Devices {
    pub(super) page: Page,
    pub(super) offset: chrono::FixedOffset,
    pub(super) name: TextField,
    pub(super) dialog: DialogState,
    pub(super) copied: Option<&'static str>,
    pub(super) tips: Tooltips,
    pub(super) places: HashMap<DeviceId, vessel_app_kit::tooltip::Place>,
}

impl Devices {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        let name = TextField::new("", |_shell: &mut Shell, _text, _window, _cx| {}, window, cx);
        Self {
            page: Page::default(),
            // Capture the viewer's offset once; pure view code does no system I/O.
            offset: *chrono::Local::now().offset(),
            name,
            dialog: DialogState::new(cx),
            copied: None,
            tips: Tooltips::new(),
            places: HashMap::new(),
        }
    }
}

impl Shell {
    /// Reconcile the selected machine before drawing, including navigation
    /// made outside Settings. A departing credential is dropped immediately.
    pub(crate) fn settle_devices(&mut self, window: &Window) {
        let target = if self.route != Route::Settings
            || self.areas.settings.section != Section::Devices
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
        let state = &mut self.areas.settings.devices;
        if state.page.target != target {
            state.copied = None;
            state.places.clear();
            state.tips = Tooltips::new();
        }
        state.page.target(target);
        if matches!(state.page.target, Target::Online(_))
            && matches!(state.page.listing, Listing::Empty)
        {
            self.devices_list();
        }
    }

    pub(super) fn devices_list(&mut self) {
        let state = &mut self.areas.settings.devices;
        let Target::Online(machine) = &state.page.target else {
            return;
        };
        let ticket = match state.page.tickets.next_ticket() {
            Ok(ticket) => ticket,
            Err(why) => {
                state.page.listing = Listing::Failed(why);
                return;
            }
        };
        state.page.listing = Listing::Loading(ticket);
        let machine = machine.clone();
        if let Err(why) = self.devices_send(machine, ticket, Ask::List) {
            self.areas
                .settings
                .devices
                .page
                .listing
                .answer(ticket, Err(why));
        }
    }

    pub(crate) fn devices_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.devices;
        match answer {
            Answer::Listed(ticket, result) => {
                state.page.listing.answer(ticket, result);
                if let Listing::Ready(devices) = &state.page.listing {
                    state.places.retain(|id, _place| {
                        devices
                            .iter()
                            .any(|device| device.id == *id && device.revoked_at.is_none())
                    });
                    for device in devices.iter().filter(|device| device.revoked_at.is_none()) {
                        let _place = state
                            .places
                            .entry(device.id)
                            .or_insert_with(|| Rc::new(Cell::new(None)));
                    }
                    state.tips = Tooltips::new();
                }
            }
            Answer::Invited(ticket, result) => {
                if state.page.inviting.answer(ticket, result) {
                    if let vessel_app_view::devices::Inviting::Ready(pairing) = &state.page.inviting
                    {
                        invitation::clock(pairing.expires(), cx);
                    }
                    self.devices_list();
                }
            }
            Answer::Revoked(ticket, result) => {
                if state.page.revoking.answer(ticket, result) {
                    self.devices_list();
                }
            }
        }
    }
}

pub(super) fn render(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    rows::render(shell, tokens, compact, cx)
}

pub(super) fn dialogs(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    revoke::dialog(shell, tokens, window, cx)
}

pub(super) fn start(shell: &Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    invitation::start(shell, window, cx);
}
