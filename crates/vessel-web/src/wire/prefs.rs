//! The page's memory across launches (ADR 0028): its preferences and its
//! composers' drafts, kept by the bridge, and the ops and the pushed frame
//! that carry them.
//!
//! The page's origin is new on every launch (ADR 0017), so nothing it
//! keeps in the browser outlives `vessel ui`. It asks the bridge instead:
//! [`Op::Preferences`](super::Op::Preferences) and
//! [`Op::Prefer`](super::Op::Prefer) read and change the preferences, and
//! every state the bridge publishes is pushed to every tab as
//! [`ServerFrame::Preferences`](super::ServerFrame::Preferences);
//! [`Op::Drafts`](super::Op::Drafts), [`Op::Draft`](super::Op::Draft) and
//! [`Op::SaveDraft`](super::Op::SaveDraft) read and write the drafts, which
//! are never pushed.
//!
//! The page sees typed values only. What the bridge does not know in the
//! file (a key a newer page wrote, a value it cannot read) stays in the
//! file and never reaches the page; that is the preferences task's.
//!
//! [Decision Log]
//! - Purpose: a theme, a text size, a rebound key or a half-written prompt
//!   that vanishes at every launch.
//! - Choice: one change names one setting ([`PreferenceChange`], tagged
//!   `set` with its value in `value`), so two tabs changing different
//!   settings both keep their change, and the answer is the whole document
//!   with a [`Revision`] the page uses to drop a state older than the one
//!   it shows.
//! - Alternatives: the page sending its whole document (two tabs would
//!   overwrite each other's settings); a JSON patch (untyped, AGENTS.md
//!   section 2).
//! - Every bound the contract names is a type here, checked where the
//!   frame is decoded: a size out of range, a key binding the grammar
//!   refuses, more edits, keys or choices than allowed do not decode, and
//!   the op is answered `unreadable` by id. The counts that depend on the
//!   document (64 actions, 32 machines) are the preferences task's.

mod bounded;
mod change;
mod drafts;
mod keys;
mod labels;
mod usage;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::{HarnessKind, NodeId, ProfileName};

pub use bounded::{
    AgentKind, Bounded, ChoiceKey, ChoiceKeyTooLong, Choices, ChoicesError, CodeSize,
    InterfaceSize, MAX_CHOICE_TEXT, MAX_CHOICES, OutOfRange, TooMany,
};
pub use change::{PreferenceChange, ShortcutEdit, ShortcutTarget};
pub use drafts::{Draft, DraftKey, DraftText, DraftTooLong, DraftsState, MAX_DRAFT};
pub use keys::{ActionId, InvalidActionId, InvalidKeyBinding, KeyBinding};
use labels::no_picks;
pub use labels::{LabelPick, LabelPicks};
pub use usage::{MAX_USAGE_PINS, UsageDisplay, UsagePin, UsagePins, UsagePreferences};

/// The most keys bound to one action.
pub const MAX_BINDINGS: usize = 2;

/// The most edits in one [`PreferenceChange::Shortcuts`].
pub const MAX_EDITS: usize = 16;

/// The most actions with keys of their own.
pub const MAX_ACTIONS: usize = 64;

/// The most machines with remembered defaults.
pub const MAX_MACHINES: usize = 32;

/// The keys bound to one action: at most [`MAX_BINDINGS`]; empty is "no
/// keys" (the action's defaults are not used either).
pub type Bindings = Bounded<KeyBinding, MAX_BINDINGS>;

/// One change's shortcut edits: at most [`MAX_EDITS`].
pub type ShortcutEdits = Bounded<ShortcutEdit, MAX_EDITS>;

/// Where the preferences are now, with every value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PreferencesState {
    /// Rises by one with every state the bridge publishes in this launch;
    /// the page drops a reply or a push older than what it shows.
    pub revision: Revision,
    /// Where they are kept.
    pub kept: Kept,
    /// The values.
    pub values: Preferences,
}

/// The number of a published preferences state within one launch, from
/// 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    /// The first state of a launch.
    pub const FIRST: Self = Self(1);

    /// The state after this one. Saturating: a launch would need 2^64
    /// changes to reach the end, and the page then keeps the last state.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Where the preferences or the drafts are kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kept {
    /// In a file, read again before every change.
    File {
        /// The file, as display text.
        path: String,
    },
    /// In the bridge's memory, until `vessel ui` stops.
    ThisLaunch {
        /// Why not in a file.
        why: LaunchOnly,
    },
}

/// Why something is kept for this launch only, for the page to say
/// without reading words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaunchOnly {
    /// This machine gives the page no place to keep them.
    NoPlace,
    /// Where to keep them could not be worked out (no home directory, say).
    NoPath {
        /// Why.
        message: String,
    },
    /// The file could not be read or written (too large, not JSON, not a
    /// regular file, an I/O error); it is not touched again in this launch.
    Unreadable {
        /// The file, as display text.
        path: String,
        /// Why.
        message: String,
    },
    /// The file was written by a newer `vessel` in a format this one would
    /// misread: its contents are used for this launch and never written.
    Newer {
        /// The file, as display text.
        path: String,
        /// Its format's version.
        version: u32,
    },
}

/// Every preference. A section or a field the file does not have is its
/// default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Preferences {
    /// How the page looks.
    pub appearance: Appearance,
    /// Which panels show.
    pub layout: Layout,
    /// The composer.
    pub composer: Composer,
    /// Keys the person chose, by action: an action not here has its
    /// default keys; one with an empty list has none. At most
    /// [`MAX_ACTIONS`].
    #[cfg_attr(feature = "ts", ts(as = "BTreeMap<ActionId, Vec<KeyBinding>>"))]
    pub shortcuts: BTreeMap<ActionId, Bindings>,
    /// How the page tells about sessions.
    pub notifications: Notifications,
    /// How the sidebar lists sessions.
    pub sidebar: SidebarView,
    /// The usage cards, summary and modal (ADR 0107).
    pub usage: UsagePreferences,
    /// What new sessions start with, by machine. At most [`MAX_MACHINES`].
    pub defaults: BTreeMap<NodeId, MachineDefaults>,
}

/// How the page looks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Appearance {
    /// Light, dark, or the system's.
    pub theme: Theme,
    /// The interface's text size.
    pub interface_size: InterfaceSize,
    /// Code's text size.
    pub code_size: CodeSize,
    /// How wide the conversation is.
    pub content_width: ContentWidth,
    /// How a conversation shows tool calls.
    pub conversation: ConversationDisplay,
}

/// The page's colours.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Whatever the system prefers, live.
    #[default]
    System,
    /// Light.
    Light,
    /// Dark.
    Dark,
}

/// How wide the conversation is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ContentWidth {
    /// 768 pixels.
    #[default]
    Comfortable,
    /// 1152 pixels.
    Wide,
    /// The window's width.
    Full,
}

/// How a conversation shows tool calls.
///
/// Its name says display because the bare word is taken: the protocol's
/// type of that name is a session's queue, commands and questions
/// (protocol 10), and the page's two generated files share one namespace
/// of type names. The field (`conversation`) and the change's tag keep
/// their names, so a preferences file written before the rename reads as
/// it did. (The bare name is not written here either: the generator
/// imports from the protocol's file every protocol type a declaration
/// mentions, its comment included.)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConversationDisplay {
    /// Every tool call open.
    #[default]
    Full,
    /// Tool calls folded to a line.
    CollapseTools,
}

/// Which panels show.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Layout {
    /// The sidebar.
    pub sidebar: Panel,
    /// The inspector (a session's summary).
    pub inspector: Panel,
}

/// Whether a panel shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Panel {
    /// Shown.
    #[default]
    Shown,
    /// Hidden.
    Hidden,
}

/// The composer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Composer {
    /// Which key sends.
    pub send: SendKey,
    /// What Send does while a turn runs.
    pub delivery: TurnDelivery,
}

/// Which key sends a prompt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SendKey {
    /// Enter sends; Shift+Enter starts a line.
    #[default]
    Enter,
    /// Mod+Enter sends; Enter starts a line.
    ModEnter,
}

/// What Send does with a prompt while the agent's turn runs: protocol
/// 10's ways of delivering one, less `start` (what Send does between
/// turns).
///
/// [Decision Log]
/// - Purpose: agent-orchestrator, GOAT and Paseo each let a person choose
///   whether a message typed during a turn waits for it, joins it or
///   stops it; GOAT and T3 Code keep the default as a setting.
/// - Choice: the default kept here, with the bridge's other preferences
///   (ADR 0028), so every tab and every launch sends the same way. Its
///   name is not the protocol's for the same set: the page's two
///   generated files share one namespace of type names, and the
///   protocol's has `start` too. (Nor is that name written here: the
///   generator imports every protocol type a comment mentions.)
/// - Steer stays a choice even for an agent that cannot steer: the page
///   then queues and says so, and the person's choice holds for the next
///   agent that can.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TurnDelivery {
    /// It waits in the session's queue for the turn to end.
    #[default]
    Queue,
    /// It joins the running turn, where the agent can steer.
    Steer,
    /// The turn is stopped and it starts the next.
    Interrupt,
}

/// How the page tells about sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Notifications {
    /// How a session that needs the person, or finished, is announced.
    pub alerts: Alerts,
    /// A count of notices in the tab's title.
    pub title: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            alerts: Alerts::default(),
            title: true,
        }
    }
}

/// How a notice is announced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Alerts {
    /// Not at all.
    #[default]
    Off,
    /// A system notification.
    System,
    /// A sound.
    Sound,
    /// Both.
    SystemAndSound,
}

/// How the sidebar lists sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SidebarView {
    /// What sessions are grouped by.
    pub group: Grouping,
    /// Their order within a group.
    pub sort: SessionSort,
    /// Whether ended sessions are listed.
    pub ended: bool,
    /// The workspace labels the sidebar is filtered by, when grouped by
    /// workspace; empty lists every workspace, `""` is Unlabelled. At most
    /// 32 names.
    #[serde(default = "no_picks")]
    #[cfg_attr(feature = "ts", ts(as = "Vec<String>"))]
    pub labels: LabelPicks,
    /// Whether the footer shows the usage summary (Paseo's footer item,
    /// off by default; ADR 0107).
    pub usage: bool,
}

impl Default for SidebarView {
    fn default() -> Self {
        // Ending a process does not archive its conversation. Hiding every
        // ended session makes a machine's retained work appear to vanish.
        Self {
            group: Grouping::default(),
            sort: SessionSort::default(),
            ended: true,
            labels: no_picks(),
            usage: false,
        }
    }
}

/// What the sidebar groups sessions by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    /// Their machine.
    #[default]
    Machine,
    /// Their status.
    Status,
    /// Their registry workspace (ADR 0106), each with its label chips;
    /// only this grouping is filtered by labels.
    Workspace,
}

/// The order of sessions within a group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SessionSort {
    /// Newest first, by creation.
    #[default]
    Created,
    /// Most recently active first.
    Activity,
}

/// What a new session on one machine starts with, remembered from the
/// last one started there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MachineDefaults {
    /// The agent picked first.
    pub agent: Option<HarnessKind>,
    /// What each agent starts with.
    #[cfg_attr(feature = "ts", ts(as = "BTreeMap<HarnessKind, AgentDefaults>"))]
    pub agents: BTreeMap<AgentKind, AgentDefaults>,
}

/// What one agent starts with on one machine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentDefaults {
    /// The profile; `None` is the harness's default.
    pub profile: Option<ProfileName>,
    /// The agent's own options and their values, applied only while the
    /// agent still offers them. At most [`MAX_CHOICES`].
    #[cfg_attr(
        feature = "ts",
        ts(as = "BTreeMap<ChoiceKey, vessel_proto::ChoiceValue>")
    )]
    pub choices: Choices,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_contracts() {
        let values = Preferences::default();
        let text = serde_json::to_string(&values).unwrap();
        assert_eq!(
            text,
            r#"{"appearance":{"theme":"system","interface_size":14,"code_size":12,"content_width":"comfortable","conversation":"full"},"layout":{"sidebar":"shown","inspector":"shown"},"composer":{"send":"enter","delivery":"queue"},"shortcuts":{},"notifications":{"alerts":"off","title":true},"sidebar":{"group":"machine","sort":"created","ended":true,"labels":[],"usage":false},"usage":{"display":"used","host":null,"pins":null},"defaults":{}}"#
        );
        assert_eq!(serde_json::from_str::<Preferences>(&text).unwrap(), values);
    }
}
