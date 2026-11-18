//! The preferences file (`ui-preferences.json`, ADR 0028): its format,
//! what this build reads of it, and one change applied to it.
//!
//! ```json
//! { "version": 1,
//!   "appearance": { "theme": "dark" },
//!   "shortcuts": { "palette": ["Mod+K", "/"] },
//!   "defaults": { "<node id>": { "agent": "codex", "agents": { "codex": { "profile": null, "choices": {} } } } } }
//! ```
//!
//! A section or a field the file leaves out is its default, and only what
//! was set is written, so a default this build changes later reaches every
//! person who never chose otherwise. The units kept whole when they do not
//! read are a setting, a section that is not an object, one action's keys,
//! one machine's defaults, and one agent's defaults under it (an unknown
//! agent kind or choice kind included): each is written back as it was,
//! and the page gets the default for it (`lenient`).
//!
//! `version` rises only with a change an older reader would misread; a
//! file with a higher one is read for this launch and never written
//! ([`Decoded::Newer`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::{NodeId, RawJson};

use self::sections::{
    AppearanceFile, ComposerFile, LayoutFile, MachineFile, NotificationsFile, SidebarFile,
    StoredAgent, UsageFile,
};
use super::lenient::{Blank, Entries, Field};
use crate::wire::{
    ActionId, AgentDefaults, AgentKind, Bindings, PreferenceChange, Preferences, ShortcutTarget,
};

/// The file's format.
pub(crate) const VERSION: u32 = 1;

/// The largest file read or written, in bytes. The bounds on the settings
/// keep every known one far under it (under 100 KiB in all).
pub(crate) const MAX_FILE: u64 = 256 * 1024;

/// The file's sections, as this build reads them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Body {
    #[serde(skip_serializing_if = "Field::is_blank")]
    appearance: Field<AppearanceFile>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    layout: Field<LayoutFile>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    composer: Field<ComposerFile>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    shortcuts: Field<Entries<ActionId, Bindings>>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    notifications: Field<NotificationsFile>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    sidebar: Field<SidebarFile>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    usage: Field<UsageFile>,
    #[serde(skip_serializing_if = "Field::is_blank")]
    defaults: Field<Entries<NodeId, MachineFile>>,
    /// Keys this build does not know, `version` taken out.
    #[serde(flatten)]
    rest: BTreeMap<String, RawJson>,
}

/// What is written: the version first, then the body.
#[derive(Serialize)]
struct Written<'a> {
    version: u32,
    #[serde(flatten)]
    body: &'a Body,
}

/// Just the version, read before the rest.
#[derive(Deserialize)]
struct Head {
    #[serde(default)]
    version: Option<u64>,
}

/// The preferences document: the values this build reads and everything
/// else the file held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PrefsDoc {
    body: Body,
}

/// A file read.
#[derive(Debug)]
pub(crate) enum Decoded {
    /// In this build's format.
    Current(PrefsDoc),
    /// In a newer format: what this build could read of it, for this
    /// launch only.
    Newer {
        /// The file's version.
        version: u32,
        /// What could be read.
        doc: PrefsDoc,
    },
}

/// Why a file is not a preferences document.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DecodeError {
    /// Not a JSON object, or its `version` is not a whole number.
    #[error("not a preferences file: {0}")]
    Format(#[source] serde_json::Error),
    /// A version no `vessel` wrote.
    #[error("version {0} is not a version of this file")]
    Version(u64),
}

/// Why a change was refused; nothing was changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Refused {
    /// It would keep keys for more than [`crate::wire::MAX_ACTIONS`]
    /// actions.
    #[error("keys are kept for at most {} actions", crate::wire::MAX_ACTIONS)]
    Actions,
    /// It would remember more than [`crate::wire::MAX_MACHINES`] machines.
    #[error("defaults are kept for at most {} machines", crate::wire::MAX_MACHINES)]
    Machines,
    /// The file would pass [`MAX_FILE`] bytes.
    #[error("the preferences file would pass {MAX_FILE} bytes")]
    Size,
}

impl PrefsDoc {
    /// Read a file's bytes.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Decoded, DecodeError> {
        let head: Head = serde_json::from_slice(bytes).map_err(DecodeError::Format)?;
        let mut body: Body = serde_json::from_slice(bytes).map_err(DecodeError::Format)?;
        let _version = body.rest.remove("version");
        let doc = Self { body };
        match head.version {
            None | Some(1) => Ok(Decoded::Current(doc)),
            Some(version @ 2..) => Ok(Decoded::Newer {
                version: u32::try_from(version).unwrap_or(u32::MAX),
                doc,
            }),
            Some(version @ 0) => Err(DecodeError::Version(version)),
        }
    }

    /// The file's bytes: pretty, since a person may read or fix it.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = serde_json::to_vec_pretty(&Written {
            version: VERSION,
            body: &self.body,
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// The values the page sees.
    pub(crate) fn values(&self) -> Preferences {
        let body = &self.body;
        Preferences {
            appearance: body
                .appearance
                .read()
                .map(AppearanceFile::values)
                .unwrap_or_default(),
            layout: body
                .layout
                .read()
                .map(LayoutFile::values)
                .unwrap_or_default(),
            composer: body
                .composer
                .read()
                .map(ComposerFile::values)
                .unwrap_or_default(),
            shortcuts: body
                .shortcuts
                .read()
                .map(|shortcuts| shortcuts.read().clone())
                .unwrap_or_default(),
            notifications: body
                .notifications
                .read()
                .map(NotificationsFile::values)
                .unwrap_or_default(),
            sidebar: body
                .sidebar
                .read()
                .map(SidebarFile::values)
                .unwrap_or_default(),
            usage: body.usage.read().map(UsageFile::values).unwrap_or_default(),
            defaults: body
                .defaults
                .read()
                .map(|machines| {
                    machines
                        .read()
                        .iter()
                        .map(|(node, machine)| (*node, machine.values()))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// The document after `change`, checked against the bounds. A change
    /// that does not make the document larger (in actions, machines or
    /// bytes) always succeeds, so a file a newer page filled past them can
    /// still be changed.
    pub(crate) fn changed(&self, change: &PreferenceChange) -> Result<Self, Refused> {
        let mut next = self.clone();
        next.apply(change);
        let grew = |before: usize, after: usize, max: usize| after > max && after > before;
        if grew(self.actions(), next.actions(), crate::wire::MAX_ACTIONS) {
            return Err(Refused::Actions);
        }
        if grew(self.machines(), next.machines(), crate::wire::MAX_MACHINES) {
            return Err(Refused::Machines);
        }
        let size = |doc: &Self| doc.encode().map_or(usize::MAX, |bytes| bytes.len());
        let bound = usize::try_from(MAX_FILE).unwrap_or(usize::MAX);
        if grew(size(self), size(&next), bound) {
            return Err(Refused::Size);
        }
        Ok(next)
    }

    /// Actions with keys kept (read or not).
    fn actions(&self) -> usize {
        self.body.shortcuts.read().map_or(0, Entries::len)
    }

    /// Machines with defaults kept (read or not).
    fn machines(&self) -> usize {
        self.body.defaults.read().map_or(0, Entries::len)
    }

    /// Apply `change`, replacing only the setting it names (and what was
    /// kept under it).
    fn apply(&mut self, change: &PreferenceChange) {
        let body = &mut self.body;
        match change {
            PreferenceChange::Theme(theme) => body.appearance.make_read().theme.set(*theme),
            PreferenceChange::InterfaceSize(size) => {
                body.appearance.make_read().interface_size.set(*size);
            }
            PreferenceChange::CodeSize(size) => body.appearance.make_read().code_size.set(*size),
            PreferenceChange::ContentWidth(width) => {
                body.appearance.make_read().content_width.set(*width);
            }
            PreferenceChange::Conversation(conversation) => {
                body.appearance.make_read().conversation.set(*conversation);
            }
            PreferenceChange::SidebarPanel(panel) => body.layout.make_read().sidebar.set(*panel),
            PreferenceChange::InspectorPanel(panel) => {
                body.layout.make_read().inspector.set(*panel);
            }
            PreferenceChange::Send(send) => body.composer.make_read().send.set(*send),
            PreferenceChange::Delivery(delivery) => {
                body.composer.make_read().delivery.set(*delivery);
            }
            PreferenceChange::Alerts(alerts) => body.notifications.make_read().alerts.set(*alerts),
            PreferenceChange::TitleCount(on) => body.notifications.make_read().title.set(*on),
            PreferenceChange::SidebarGroup(group) => body.sidebar.make_read().group.set(*group),
            PreferenceChange::SidebarSort(sort) => body.sidebar.make_read().sort.set(*sort),
            PreferenceChange::SidebarEnded(on) => body.sidebar.make_read().ended.set(*on),
            PreferenceChange::SidebarLabels { picks } => {
                body.sidebar.make_read().labels.set(picks.clone());
            }
            PreferenceChange::SidebarUsage(on) => body.sidebar.make_read().usage.set(*on),
            PreferenceChange::UsageDisplay(display) => {
                body.usage.make_read().display.set(*display);
            }
            PreferenceChange::UsageHost(host) => body.usage.make_read().host.set(*host),
            PreferenceChange::UsagePins(pins) => body.usage.make_read().pins.set(pins.clone()),
            PreferenceChange::Shortcuts { edits } => {
                let shortcuts = body.shortcuts.make_read();
                for edit in edits.items() {
                    match &edit.to {
                        ShortcutTarget::Bindings { keys } => {
                            shortcuts.insert(edit.action.clone(), keys.clone());
                        }
                        ShortcutTarget::Default => shortcuts.remove(&edit.action),
                    }
                }
            }
            PreferenceChange::ResetShortcuts => body.shortcuts = Field::default(),
            PreferenceChange::DefaultAgent { node, agent } => {
                let machines = body.defaults.make_read();
                let machine = machines.make_read(*node);
                match agent {
                    Some(agent) => machine.agent.set(Some(*agent)),
                    None => machine.agent = Field::default(),
                }
                machines.remove_if(node, Blank::is_blank);
            }
            PreferenceChange::AgentDefaults {
                node,
                agent,
                profile,
                choices,
            } => {
                let stored = StoredAgent(AgentDefaults {
                    profile: profile.clone(),
                    choices: choices.clone(),
                });
                let machine = body.defaults.make_read().make_read(*node);
                machine.agents.make_read().insert(AgentKind(*agent), stored);
            }
            PreferenceChange::ForgetDefaults { node } => {
                // A `defaults` that is not an object holds no node this
                // build could name: left as it is.
                if body.defaults.read().is_some() {
                    body.defaults.make_read().remove(node);
                }
            }
        }
    }
}

mod sections;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod usage_tests;
