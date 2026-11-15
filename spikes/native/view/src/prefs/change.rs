//! One change to the preferences (the bridge's `wire/prefs/change.rs` and
//! the page's `prefs-change.ts`): each names one setting and its new
//! value, so two surfaces changing different settings never undo each
//! other.

use serde::{Deserialize, Serialize};
use vessel_proto::{HarnessKind, NodeId, ProfileName};

use super::{
    ActionId, AgentDefaults, AgentKind, Alerts, Animations, Bindings, Choices, CodeSize,
    ContentWidth, ConversationDisplay, Grouping, InterfaceSize, MAX_ACTIONS, MAX_EDITS,
    MAX_MACHINES, MachineDefaults, Panel, Preferences, SendKey, SessionSort, Theme, TooMany,
    TurnDelivery,
};

/// One change: the setting it names, and its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "set", content = "value", rename_all = "snake_case")]
pub enum PreferenceChange {
    /// The app language; applies at once on every screen.
    Language(crate::localization::LanguagePreference),
    /// The theme.
    Theme(Theme),
    /// The interface's text size.
    InterfaceSize(InterfaceSize),
    /// Code's text size.
    CodeSize(CodeSize),
    /// The conversation's width.
    ContentWidth(ContentWidth),
    /// How a conversation shows tool calls.
    Conversation(ConversationDisplay),
    /// Whether the app moves.
    Animations(Animations),
    /// Whether the sidebar shows.
    SidebarPanel(Panel),
    /// Whether the inspector shows.
    InspectorPanel(Panel),
    /// Which key sends.
    Send(SendKey),
    /// What Send does while a turn runs.
    Delivery(TurnDelivery),
    /// How notices are announced.
    Alerts(Alerts),
    /// Whether the window's title counts notices.
    TitleCount(bool),
    /// What the sidebar groups by.
    SidebarGroup(Grouping),
    /// The sidebar's order.
    SidebarSort(SessionSort),
    /// Whether the sidebar lists ended sessions.
    SidebarEnded(bool),
    /// Some actions' keys.
    Shortcuts {
        /// The edits, applied in order.
        edits: ShortcutEdits,
    },
    /// Every action back to its default keys.
    ResetShortcuts,
    /// The agent a machine's new session picks first.
    DefaultAgent {
        /// The machine.
        node: NodeId,
        /// The agent; `None` forgets it.
        agent: Option<HarnessKind>,
    },
    /// What one agent starts with on one machine.
    AgentDefaults {
        /// The machine.
        node: NodeId,
        /// The agent.
        agent: HarnessKind,
        /// The profile; `None` is the agent's own.
        profile: Option<ProfileName>,
        /// Its options' values.
        choices: Choices,
    },
    /// Forget everything kept for one machine.
    ForgetDefaults {
        /// The machine.
        node: NodeId,
    },
}

/// One action's new keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutEdit {
    /// The action.
    pub action: ActionId,
    /// Its keys from now on.
    pub to: ShortcutTarget,
}

/// What an action's keys become.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShortcutTarget {
    /// These keys (none is "no keys").
    Bindings {
        /// The keys.
        keys: Bindings,
    },
    /// The action's default keys.
    Default,
}

/// At most [`MAX_EDITS`] shortcut edits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<ShortcutEdit>", into = "Vec<ShortcutEdit>")]
pub struct ShortcutEdits(Vec<ShortcutEdit>);

impl ShortcutEdits {
    /// The edits, in order.
    #[must_use]
    pub fn items(&self) -> &[ShortcutEdit] {
        &self.0
    }
}

impl TryFrom<Vec<ShortcutEdit>> for ShortcutEdits {
    type Error = TooMany;

    fn try_from(edits: Vec<ShortcutEdit>) -> Result<Self, TooMany> {
        if edits.len() <= MAX_EDITS {
            Ok(Self(edits))
        } else {
            Err(TooMany { max: MAX_EDITS })
        }
    }
}

impl From<ShortcutEdits> for Vec<ShortcutEdit> {
    fn from(edits: ShortcutEdits) -> Self {
        edits.0
    }
}

/// Why a change was refused; nothing was changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// It would keep keys for more than [`MAX_ACTIONS`] actions.
    #[error("keys are kept for at most {MAX_ACTIONS} actions")]
    Actions,
    /// It would remember more than [`MAX_MACHINES`] machines.
    #[error("defaults are kept for at most {MAX_MACHINES} machines")]
    Machines,
}

impl Preferences {
    /// The preferences after `change` (the page's `applyChange`), checked
    /// against the bounds: a change that would grow the shortcuts past
    /// [`MAX_ACTIONS`] or the machines past [`MAX_MACHINES`] is refused; one
    /// that does not grow them always applies, so a document filled past
    /// them by a newer Vessel can still be changed.
    ///
    /// # Errors
    ///
    /// [`Refused`] when the change would pass a bound.
    pub fn changed(&self, change: &PreferenceChange) -> Result<Self, Refused> {
        let mut next = self.clone();
        next.apply(change);
        let grew = |before: usize, after: usize, max: usize| after > max && after > before;
        if grew(self.shortcuts.len(), next.shortcuts.len(), MAX_ACTIONS) {
            return Err(Refused::Actions);
        }
        if grew(self.defaults.len(), next.defaults.len(), MAX_MACHINES) {
            return Err(Refused::Machines);
        }
        Ok(next)
    }

    /// Apply `change`, replacing only the setting it names.
    fn apply(&mut self, change: &PreferenceChange) {
        match change {
            PreferenceChange::Language(language) => self.language = *language,
            PreferenceChange::Theme(theme) => self.appearance.theme = *theme,
            PreferenceChange::InterfaceSize(size) => self.appearance.interface_size = *size,
            PreferenceChange::CodeSize(size) => self.appearance.code_size = *size,
            PreferenceChange::ContentWidth(width) => self.appearance.content_width = *width,
            PreferenceChange::Conversation(display) => self.appearance.conversation = *display,
            PreferenceChange::Animations(animations) => self.appearance.animations = *animations,
            PreferenceChange::SidebarPanel(panel) => self.layout.sidebar = *panel,
            PreferenceChange::InspectorPanel(panel) => self.layout.inspector = *panel,
            PreferenceChange::Send(send) => self.composer.send = *send,
            PreferenceChange::Delivery(delivery) => self.composer.delivery = *delivery,
            PreferenceChange::Alerts(alerts) => self.notifications.alerts = *alerts,
            PreferenceChange::TitleCount(title) => self.notifications.title = *title,
            PreferenceChange::SidebarGroup(group) => self.sidebar.group = *group,
            PreferenceChange::SidebarSort(sort) => self.sidebar.sort = *sort,
            PreferenceChange::SidebarEnded(ended) => self.sidebar.ended = *ended,
            PreferenceChange::Shortcuts { edits } => {
                for edit in edits.items() {
                    match &edit.to {
                        ShortcutTarget::Bindings { keys } => {
                            let _before = self.shortcuts.insert(edit.action.clone(), keys.clone());
                        }
                        ShortcutTarget::Default => {
                            let _before = self.shortcuts.remove(&edit.action);
                        }
                    }
                }
            }
            PreferenceChange::ResetShortcuts => self.shortcuts.clear(),
            PreferenceChange::DefaultAgent { node, agent } => {
                self.defaults.entry(*node).or_default().agent = *agent;
            }
            PreferenceChange::AgentDefaults {
                node,
                agent,
                profile,
                choices,
            } => {
                let machine: &mut MachineDefaults = self.defaults.entry(*node).or_default();
                let _before = machine.agents.insert(
                    AgentKind(*agent),
                    AgentDefaults {
                        profile: profile.clone(),
                        choices: choices.clone(),
                    },
                );
            }
            PreferenceChange::ForgetDefaults { node } => {
                let _before = self.defaults.remove(node);
            }
        }
    }
}
