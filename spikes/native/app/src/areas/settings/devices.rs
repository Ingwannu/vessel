//! Settings > Devices, on the existing settings frame and kit only.
//! Pure state and credential policy live in `view::devices`; this area
//! holds field/focus handles and wires their actions to `link::devices`.

mod authority;
mod inspection;
mod invitation;
mod revoke;
mod row;
mod rows;

use std::{cell::Cell, collections::HashMap, rc::Rc};
use vessel_app_kit::ui::{AnyElement, Context, Window};
use vessel_app_kit::{DialogState, TextField, Tooltips};
use vessel_app_link::devices::{Answer, Ask, Outcome};
use vessel_app_view::devices::{
    Authority, ConnectionContext, IdentityChange, Listing, MachineKey, Page, Target,
};
use vessel_app_view::settings::Section;
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
    fn bind(&mut self, context: Option<ConnectionContext>) {
        if self.page.bind(context) == IdentityChange::Replaced {
            self.copied = None;
            self.places.clear();
            self.tips = Tooltips::new();
        }
    }
}

impl Shell {
    /// Checked device dispatch keeps a refused queue from leaving its page pending.
    pub(crate) fn devices_send(
        &self,
        context: ConnectionContext,
        ticket: vessel_app_view::devices::Ticket,
        ask: Ask,
    ) -> Result<(), vessel_app_view::devices::Problem> {
        if self.route != Route::Settings
            || self.areas.settings.section != Section::Devices
            || super::choices::chosen_machine(self)
                .is_none_or(|machine| machine.name != context.machine().as_str())
            || self
                .areas
                .settings
                .devices
                .page
                .check_context(&context)
                .is_err()
        {
            return Err(vessel_app_view::devices::Problem::TargetChanged);
        }
        if !matches!(ask, Ask::Authority)
            && !self.areas.settings.devices.page.authority().may_manage()
        {
            return Err(vessel_app_view::devices::Problem::AccessNotConfirmed);
        }
        self.devices_dispatch(context, ticket, ask)
    }

    /// A single Machine snapshot supplies both welcome identity and epoch.
    pub(crate) fn devices_fleet(&mut self) {
        let context = super::choices::chosen_machine(self)
            .and_then(vessel_app_view::Machine::connection_context);
        self.areas.settings.devices.bind(context);
    }

    /// Reconcile navigation as well as authenticated events before a callback,
    /// including a press delivered before the next frame has been drawn.
    pub(super) fn devices_press(&mut self, context: &ConnectionContext, window: &Window) -> bool {
        self.settle_devices(window);
        self.areas
            .settings
            .devices
            .page
            .check_context(context)
            .is_ok()
            && self.areas.settings.devices.page.authority().may_manage()
    }

    pub(crate) fn devices_retired(
        &mut self,
        tag: &vessel_app_view::connection::ConnectionTag,
    ) -> bool {
        let mut changed = false;
        for machine in &mut self.fleet.machines {
            if machine.name == tag.machine().as_str()
                && machine.connection_epoch == Some(tag.epoch())
            {
                machine.connection_epoch = None;
                changed = true;
            }
        }
        let state = &mut self.areas.settings.devices;
        if state
            .page
            .context()
            .is_some_and(|context| context.tag() == *tag)
        {
            state.bind(None);
            changed = true;
        }
        changed
    }

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
                    if machine.connection_context().is_some() {
                        Target::Online(key)
                    } else {
                        Target::Offline(key)
                    }
                }
            }
        };
        let context = super::choices::chosen_machine(self)
            .and_then(vessel_app_view::Machine::connection_context);
        let state = &mut self.areas.settings.devices;
        if state.page.target != target {
            state.copied = None;
            state.places.clear();
            state.tips = Tooltips::new();
        }
        state.page.target(target);
        state.bind(context);
        if matches!(state.page.target, Target::Online(_))
            && matches!(state.page.authority(), Authority::Unchecked)
        {
            self.devices_authority();
        }
    }

    pub(super) fn devices_list(&mut self) {
        let state = &mut self.areas.settings.devices;
        if !state.page.authority().may_manage() {
            return;
        }
        let Some(context) = state.page.context().cloned() else {
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
        if let Err(why) = self.devices_send(context, ticket, Ask::List) {
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
        if state.page.check_context(&answer.context).is_err() {
            return;
        }
        match answer.outcome {
            Outcome::Authority(ticket, result) => {
                if state.page.authority_answer(&answer.context, ticket, result)
                    && state.page.authority().may_manage()
                {
                    self.devices_list();
                }
            }
            Outcome::Listed(ticket, result) => {
                if !state.page.listed(&answer.context, ticket, result) {
                    return;
                }
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
            Outcome::Invited(ticket, result) => {
                if state.page.invited(&answer.context, ticket, result) {
                    if let vessel_app_view::devices::Inviting::Ready(pairing) = &state.page.inviting
                    {
                        invitation::clock(pairing.expires(), cx);
                    }
                    self.devices_list();
                }
            }
            Outcome::Revoked(ticket, result) => {
                if state.page.revoked(&answer.context, ticket, result) {
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
        .or_else(|| inspection::dialog(shell, tokens, window, cx))
}

pub(super) fn start(shell: &Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    invitation::start(shell, window, cx);
}
