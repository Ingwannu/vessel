//! The page's memory across launches (ADR 0025): its preferences and its
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
mod drafts;
mod keys;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vessel_proto::{HarnessKind, NodeId, ProfileName};

pub use bounded::{
    AgentKind, Bounded, ChoiceKey, ChoiceKeyTooLong, Choices, ChoicesError, CodeSize,
    InterfaceSize, MAX_CHOICE_TEXT, MAX_CHOICES, OutOfRange, TooMany,
};
pub use drafts::{Draft, DraftKey, DraftText, DraftTooLong, DraftsState, MAX_DRAFT};
pub use keys::{ActionId, InvalidActionId, InvalidKeyBinding, KeyBinding};

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
    pub conversation: Conversation,
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Conversation {
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SidebarView {
    /// What sessions are grouped by.
    pub group: Grouping,
    /// Their order within a group.
    pub sort: SessionSort,
    /// Whether ended sessions are listed.
    pub ended: bool,
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

/// One change to the preferences: each replaces one setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "set", content = "value", rename_all = "snake_case")]
pub enum PreferenceChange {
    /// [`Appearance::theme`].
    Theme(Theme),
    /// [`Appearance::interface_size`].
    InterfaceSize(InterfaceSize),
    /// [`Appearance::code_size`].
    CodeSize(CodeSize),
    /// [`Appearance::content_width`].
    ContentWidth(ContentWidth),
    /// [`Appearance::conversation`].
    Conversation(Conversation),
    /// [`Layout::sidebar`].
    SidebarPanel(Panel),
    /// [`Layout::inspector`].
    InspectorPanel(Panel),
    /// [`Composer::send`].
    Send(SendKey),
    /// [`Notifications::alerts`].
    Alerts(Alerts),
    /// [`Notifications::title`].
    TitleCount(bool),
    /// [`SidebarView::group`].
    SidebarGroup(Grouping),
    /// [`SidebarView::sort`].
    SidebarSort(SessionSort),
    /// [`SidebarView::ended`].
    SidebarEnded(bool),
    /// Keys for some actions, applied together, so a key moves from one
    /// action to another in one change.
    Shortcuts {
        /// At most [`MAX_EDITS`].
        #[cfg_attr(feature = "ts", ts(as = "Vec<ShortcutEdit>"))]
        edits: ShortcutEdits,
    },
    /// Every action back to its default keys (actions this page does not
    /// know included).
    ResetShortcuts,
    /// The agent a new session on `node` picks first.
    DefaultAgent {
        /// The machine.
        node: NodeId,
        /// The agent; `None` forgets it.
        agent: Option<HarnessKind>,
    },
    /// What `agent` starts with on `node`, replacing what was remembered.
    AgentDefaults {
        /// The machine.
        node: NodeId,
        /// The agent.
        agent: HarnessKind,
        /// The profile; `None` is the harness's default.
        profile: Option<ProfileName>,
        /// The agent's options, at most [`MAX_CHOICES`]; an id or a value
        /// longer than [`MAX_CHOICE_TEXT`] bytes is not remembered.
        #[cfg_attr(
            feature = "ts",
            ts(as = "BTreeMap<ChoiceKey, vessel_proto::ChoiceValue>")
        )]
        choices: Choices,
    },
    /// Forget everything remembered for `node`.
    ForgetDefaults {
        /// The machine.
        node: NodeId,
    },
}

/// One action's new keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShortcutEdit {
    /// The action.
    pub action: ActionId,
    /// Its keys from now on.
    pub to: ShortcutTarget,
}

/// What an action's keys become.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShortcutTarget {
    /// These keys (empty: none at all).
    Bindings {
        /// At most [`MAX_BINDINGS`].
        #[cfg_attr(feature = "ts", ts(as = "Vec<KeyBinding>"))]
        keys: Bindings,
    },
    /// The action's default keys.
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_are_tagged_by_setting_with_their_value() {
        let change: PreferenceChange =
            serde_json::from_str(r#"{"set":"theme","value":"dark"}"#).unwrap();
        assert_eq!(change, PreferenceChange::Theme(Theme::Dark));
        let change: PreferenceChange =
            serde_json::from_str(r#"{"set":"reset_shortcuts"}"#).unwrap();
        assert_eq!(change, PreferenceChange::ResetShortcuts);
        let change: PreferenceChange = serde_json::from_str(
            r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["Mod+K","/"]}},{"action":"find","to":{"kind":"default"}}]}}"#,
        )
        .unwrap();
        let PreferenceChange::Shortcuts { edits } = change else {
            panic!("{change:?}");
        };
        assert_eq!(edits.items().len(), 2);
        for refused in [
            r#"{"set":"interface_size","value":21}"#,
            r#"{"set":"code_size","value":9}"#,
            r#"{"set":"theme","value":"sepia"}"#,
            r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["Mod+K","/","N"]}}]}}"#,
            r#"{"set":"shortcuts","value":{"edits":[{"action":"palette","to":{"kind":"bindings","keys":["mod+k"]}}]}}"#,
            r#"{"set":"what","value":1}"#,
        ] {
            assert!(
                serde_json::from_str::<PreferenceChange>(refused).is_err(),
                "{refused}"
            );
        }
        let seventeen: Vec<String> = (0..17)
            .map(|n| format!(r#"{{"action":"a{n}","to":{{"kind":"default"}}}}"#))
            .collect();
        let text = format!(
            r#"{{"set":"shortcuts","value":{{"edits":[{}]}}}}"#,
            seventeen.join(",")
        );
        assert!(serde_json::from_str::<PreferenceChange>(&text).is_err());
    }

    #[test]
    fn defaults_are_the_contracts() {
        let values = Preferences::default();
        let text = serde_json::to_string(&values).unwrap();
        assert_eq!(
            text,
            r#"{"appearance":{"theme":"system","interface_size":14,"code_size":12,"content_width":"comfortable","conversation":"full"},"layout":{"sidebar":"shown","inspector":"shown"},"composer":{"send":"enter"},"shortcuts":{},"notifications":{"alerts":"off","title":true},"sidebar":{"group":"machine","sort":"created","ended":false},"defaults":{}}"#
        );
        assert_eq!(serde_json::from_str::<Preferences>(&text).unwrap(), values);
    }
}
