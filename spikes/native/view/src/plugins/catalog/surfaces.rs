//! A machine's plugin screens in the app (ADR 0115, "Surfaces"), as the
//! page's `data/plugin-surfaces.ts`: which declared screen an address
//! names, the sidebar rows that open screens, merged across machines as
//! Paseo merges them (`packages/app/src/plugins/sidebar-groups.ts:33-37`,
//! `97-110`), and whether a surface holds anything a device without admin
//! cannot use.
//!
//! [Decision Log]
//! - Purpose: the sidebar, the screen route and the palette must agree on
//!   which screen a (machine, plugin, screen) names and who may act on it.
//! - Choice: pure lookups over the held catalogs, tested on their own. A
//!   screen is drawn for every device that reads the catalog; what calls a
//!   method is disabled without `may_invoke` (coordinator ruling on P3b,
//!   Q1). Settings screens wait for their own packet (N2).
//! - A row declared on several connected machines is drawn once. It opens
//!   the machine whose screen is open now, else the one last shown, else
//!   the first in the book; an away machine's rows are left out (the page's
//!   node review R4).

use std::collections::HashMap;

use vessel_proto::RawJson;
use vessel_proto::plugins::{
    Action, ButtonBehavior, ClientInstallation, ContributionContent, ContributionId,
    ControlContent, InvokeContext, InvokePlugin, MenuEntry, PluginId, RpcName, SidebarPlacement,
    Surface,
};

use crate::devices::MachineKey;

use super::{Catalogs, PluginCall};

/// A plugin screen: its machine, plugin and declared screen id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScreenAddress {
    /// The machine whose plugin declared it.
    pub machine: MachineKey,
    /// The installation.
    pub plugin: PluginId,
    /// The screen's contribution id.
    pub screen: ContributionId,
}

/// A declared screen found in a machine's last catalog.
#[derive(Debug, Clone, Copy)]
pub struct FoundScreen<'a> {
    /// The installation and its run.
    pub installation: &'a ClientInstallation,
    /// The screen's title.
    pub title: &'a str,
    /// What it draws.
    pub surface: &'a Surface,
    /// Whether this device may call the plugin's methods.
    pub may_invoke: bool,
}

/// A sidebar row a plugin declared, opening one of its screens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarRow {
    /// One per plugin, placement and item, across machines.
    pub key: String,
    /// What it says.
    pub title: String,
    /// The screen it opens.
    pub target: ScreenAddress,
}

/// How a row declared on several machines picks the one it opens.
#[derive(Debug, Clone, Copy)]
pub struct SidebarChoice<'a> {
    /// The connected machines; an away machine's rows are left out.
    pub online: &'a [MachineKey],
    /// The plugin screen open now, if any.
    pub current: Option<&'a ScreenAddress>,
    /// The machine each plugin screen was last shown from.
    pub remembered: &'a HashMap<(PluginId, ContributionId), MachineKey>,
}

impl Catalogs {
    /// The screen `at` names, if `at.machine`'s last catalog has a running
    /// plugin declaring it.
    #[must_use]
    pub fn screen(&self, at: &ScreenAddress) -> Option<FoundScreen<'_>> {
        let catalog = &self.held.get(&at.machine)?.catalog;
        let installation = catalog
            .catalog
            .installations()
            .iter()
            .find(|entry| entry.address().plugin == at.plugin)?;
        installation
            .document()
            .contributions()
            .iter()
            .find_map(|item| {
                let ContributionContent::Screen { surface } = &item.content else {
                    return None;
                };
                (item.id == at.screen).then_some(FoundScreen {
                    installation,
                    title: item.title.as_str(),
                    surface,
                    may_invoke: catalog.may_invoke,
                })
            })
    }

    /// The connected machines, in book order, whose running `plugin`
    /// declares `screen`: the screen header's host filter (Paseo's
    /// `getPluginSurfaceContributionServerIds`, surface-screen.tsx:148-153),
    /// shown when there are two or more.
    #[must_use]
    pub fn screen_machines(
        &self,
        plugin: &PluginId,
        screen: &ContributionId,
        online: &[MachineKey],
    ) -> Vec<MachineKey> {
        self.in_order()
            .map(|(machine, _)| machine)
            .filter(|machine| online.contains(machine))
            .filter(|machine| {
                self.screen(&ScreenAddress {
                    machine: (*machine).clone(),
                    plugin: plugin.clone(),
                    screen: screen.clone(),
                })
                .is_some()
            })
            .cloned()
            .collect()
    }

    /// The connected machines' `placement` rows, one per plugin and item.
    #[must_use]
    pub fn sidebar_rows(
        &self,
        placement: SidebarPlacement,
        choice: &SidebarChoice<'_>,
    ) -> Vec<SidebarRow> {
        // (plugin, item) -> (title, screen, machines), in first-seen order.
        let mut merged: Vec<(String, String, ScreenAddress, Vec<MachineKey>)> = Vec::new();
        for (machine, catalog) in self.in_order() {
            if !choice.online.contains(machine) {
                continue;
            }
            for installation in catalog.catalog.installations() {
                let plugin = &installation.address().plugin;
                for item in installation.document().contributions() {
                    let ContributionContent::Sidebar {
                        placement: placed,
                        screen,
                    } = &item.content
                    else {
                        continue;
                    };
                    if *placed != placement {
                        continue;
                    }
                    let key = format!("{plugin}:{placement:?}:{}", item.id);
                    if let Some(held) = merged.iter_mut().find(|held| held.0 == key) {
                        held.3.push(machine.clone());
                    } else {
                        let target = ScreenAddress {
                            machine: machine.clone(),
                            plugin: plugin.clone(),
                            screen: screen.clone(),
                        };
                        merged.push((
                            key,
                            item.title.as_str().to_owned(),
                            target,
                            vec![machine.clone()],
                        ));
                    }
                }
            }
        }
        merged
            .into_iter()
            .map(|(key, title, mut target, machines)| {
                target.machine = pick(&target, &machines, choice);
                SidebarRow { key, title, target }
            })
            .collect()
    }
}

/// The machine a merged row opens: the screen open now, else the one last
/// shown, else the first.
fn pick(target: &ScreenAddress, machines: &[MachineKey], choice: &SidebarChoice<'_>) -> MachineKey {
    let same = |open: &&ScreenAddress| open.plugin == target.plugin && open.screen == target.screen;
    choice
        .current
        .filter(same)
        .map(|open| &open.machine)
        .filter(|machine| machines.contains(machine))
        .or_else(|| {
            choice
                .remembered
                .get(&(target.plugin.clone(), target.screen.clone()))
                .filter(|machine| machines.contains(machine))
        })
        .unwrap_or(&target.machine)
        .clone()
}

/// Whether a device without admin would find something on `surface` it
/// cannot use: a control that calls a method. Opening a screen needs none.
#[must_use]
pub fn needs_admin(surface: &Surface) -> bool {
    surface
        .controls
        .iter()
        .any(|control| match &control.content {
            ControlContent::TextField { .. }
            | ControlContent::Choice { .. }
            | ControlContent::Toggle { .. }
            | ControlContent::SettingsSwitch { .. }
            | ControlContent::SettingsSelect { .. }
            | ControlContent::SettingsInput { .. } => true,
            ControlContent::Button { button } => behavior_needs_admin(&button.behavior),
            ControlContent::Stack { surface }
            | ControlContent::Row { surface }
            | ControlContent::Section { surface, .. }
            | ControlContent::Card { surface } => needs_admin(surface),
            ControlContent::Text { .. }
            | ControlContent::Markdown { .. }
            | ControlContent::Status { .. } => false,
        })
}

fn behavior_needs_admin(behavior: &ButtonBehavior) -> bool {
    match behavior {
        ButtonBehavior::Action { action } => action_needs_admin(action),
        ButtonBehavior::Menu { items } => items.iter().any(|entry| match entry {
            MenuEntry::Separator { .. } => false,
            MenuEntry::Item { behavior, .. } => behavior_needs_admin(behavior),
        }),
        ButtonBehavior::Popover { surface } => needs_admin(surface),
    }
}

/// Whether taking `action` needs admin: everything but opening a screen.
#[must_use]
pub const fn action_needs_admin(action: &Action) -> bool {
    match action {
        Action::OpenScreen { .. } => false,
        Action::Invoke { .. } | Action::OpenSettings { .. } => true,
    }
}

/// What a screen's control sends (ADR 0115, "What a control sends"): a
/// text field its text, a choice its option id, a toggle its new state,
/// each as `{"value": …}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlValue {
    /// A text field's text, on Enter.
    Text(String),
    /// A choice's picked option.
    Choice(ContributionId),
    /// A toggle's new state.
    Toggle(bool),
}

/// A call of `method` from the screen at `at` (a screen's context is
/// global), with `input` as declared or built by [`control_input`].
#[must_use]
pub fn screen_call(
    at: &ScreenAddress,
    installation: &ClientInstallation,
    method: &RpcName,
    input: RawJson,
) -> PluginCall {
    PluginCall {
        machine: at.machine.clone(),
        invoke: InvokePlugin {
            plugin: installation.address().plugin.clone(),
            generation: installation.address().generation,
            method: method.clone(),
            input,
            context: InvokeContext::Global,
        },
    }
}

/// `{"value": …}` for `value`.
#[must_use]
pub fn control_input(value: &ControlValue) -> RawJson {
    let value = match value {
        ControlValue::Text(text) => serde_json::Value::String(text.clone()),
        ControlValue::Choice(id) => serde_json::Value::String(id.as_str().to_owned()),
        ControlValue::Toggle(on) => serde_json::Value::Bool(*on),
    };
    let text = serde_json::json!({ "value": value }).to_string();
    RawJson::from_text(text).unwrap_or_else(|_never| RawJson::empty_object())
}
