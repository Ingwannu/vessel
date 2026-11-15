//! Pin edits snapshot the displayed defaults through the shared preference writer.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::{
    connection::ConnectionContext,
    devices::Ticket,
    prefs::PreferenceChange,
    sidebar::navigation::{Builtin, Key, Section, Visibility},
    usage::{Pin, default_pins},
};
use vessel_proto::{limits::LimitKey, usage_reports::UsageSourceId};
impl Shell {
    pub(super) fn usage_toggle_pin(
        &mut self,
        machine: ConnectionContext,
        ticket: Ticket,
        pin: Pin<UsageSourceId, LimitKey>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.usage.inventory.selection() != Some(&machine)
            || self.areas.usage.inventory.observation_ticket() != Some(ticket)
        {
            return;
        }
        let Some(inventory) = self.areas.usage.inventory.phase().observation() else {
            return;
        };
        let reports = super::sidebar::candidates(inventory, self.words);
        let defaults = default_pins(&reports);
        let mut selected = self.prefs.settings.usage.pins.clone();
        selected.toggle(pin, &defaults);
        self.prefer(&PreferenceChange::UsagePins(selected), window, cx);
    }
    pub(super) fn usage_show_sidebar(
        &mut self,
        shown: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let navigation = &self.prefs.settings.sidebar_navigation;
        let current = navigation.resolve(Section::Footer, &[]);
        let visibility = if shown {
            Visibility::Shown
        } else {
            Visibility::Hidden
        };
        let entries = navigation.entries(Section::Footer).visibility(
            &current,
            &Key::of(Builtin::Usage),
            visibility,
        );
        if let Ok(entries) = entries {
            self.prefer(
                &PreferenceChange::SidebarNavigation {
                    section: Section::Footer,
                    entries,
                },
                window,
                cx,
            );
        }
    }
}
