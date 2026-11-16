//! The person's preferences: every setting the page keeps with its bridge
//! (`web/src/data/prefs.ts`, `prefs-change.ts`; the bridge's
//! `wire/prefs.rs`), as one document with the page's defaults and the
//! page's rules for a change.
//!
//! [Decision Log]
//! - Purpose: ADR 0036 wants one person's settings to follow the person to
//!   every surface. The app keeps them in its own file today, and on a node
//!   later; either way the document is the bridge's, so a surface that
//!   reads one reads every other's.
//! - Constraint: a file written by a newer Vessel may hold a value this one
//!   does not know (a theme, a size past the range); the page then shows
//!   the default for that one setting and keeps the rest.
//! - Choice: the bridge's types and JSON, plus this app's language choice; every field is
//!   read on its own and falls back to its default when it does not parse
//!   ([`lenient`]), as the page's first paint does. A change is one
//!   [`PreferenceChange`], applied by [`Preferences::changed`] with the
//!   bridge's bounds (a change that would grow the document past them is
//!   refused; one that does not grow it always applies).
//! - Cost: the bridge also keeps what a newer page wrote and this one does
//!   not know (unknown sections and fields) and writes it back; this
//!   document drops it on the next save. Named in the track's notes.

mod appearance;
mod change;
mod keys;
mod language;
mod send;
mod usage;
pub use usage::UsagePreferences;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::str::FromStr;

use serde::de::{Deserializer, IgnoredAny};
use serde::{Deserialize, Serialize};
use vessel_proto::{ChoiceValue, HarnessKind, NodeId, OptionId, ProfileName};

pub use appearance::{
    Animations, Appearance, CodeSize, ContentWidth, ConversationDisplay, InterfaceSize, OutOfRange,
    Theme,
};
pub use change::{PreferenceChange, Refused, ShortcutEdit, ShortcutEdits, ShortcutTarget};
pub use keys::{ActionId, Bindings, InvalidActionId, InvalidKeyBinding, KeyBinding, TooMany};
pub use send::{EnterPress, send_key_label, sends, sends_other};

/// The most keys kept for one action.
pub const MAX_BINDINGS: usize = 2;
/// The most shortcut edits in one change.
pub const MAX_EDITS: usize = MAX_ACTIONS;
/// The most actions with keys kept.
pub const MAX_ACTIONS: usize = 128;
/// The most machines with defaults kept.
pub const MAX_MACHINES: usize = 32;
/// The most choices kept for one agent on one machine.
pub const MAX_CHOICES: usize = 8;
/// The longest choice id or value kept, in UTF-8 bytes.
pub const MAX_CHOICE_TEXT: usize = 64;

/// A value as the input says, or something this build does not read.
#[derive(Deserialize)]
#[serde(untagged)]
enum Maybe<T> {
    Known(T),
    Other(IgnoredAny),
}

impl<T> Maybe<T> {
    fn known(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Other(_) => None,
        }
    }
}

/// `T` as the input says, or its default when the input says something
/// else (a value a newer Vessel wrote, a size out of range).
pub(crate) fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Maybe::<T>::deserialize(deserializer)?
        .known()
        .unwrap_or_default())
}

/// A map read entry by entry: an entry whose key or value this build does
/// not read is left out, the others kept (the bridge's `Entries`).
fn lenient_entries<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: FromStr + Ord,
    V: Deserialize<'de>,
{
    let read: BTreeMap<String, Maybe<V>> = lenient(deserializer)?;
    Ok(read
        .into_iter()
        .filter_map(|(key, value)| Some((key.parse().ok()?, value.known()?)))
        .collect())
}

/// Every preference. A section or a field the input does not have, or
/// has in a form this build does not read, is its default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Usage host and display choices, independent from sidebar visibility.
    #[serde(deserialize_with = "lenient")]
    pub usage: Box<UsagePreferences>,
    /// The app language, or the operating system when System.
    #[serde(deserialize_with = "lenient")]
    pub language: crate::localization::LanguagePreference,
    /// How the app looks.
    #[serde(deserialize_with = "lenient")]
    pub appearance: Appearance,
    /// Which panels show.
    #[serde(deserialize_with = "lenient")]
    pub layout: Layout,
    /// The composer.
    #[serde(deserialize_with = "lenient")]
    pub composer: Composer,
    /// Keys the person chose, by action: an action not here has its
    /// default keys; one with an empty list has none.
    #[serde(deserialize_with = "lenient_entries")]
    pub shortcuts: BTreeMap<ActionId, Bindings>,
    /// How sessions are announced.
    #[serde(deserialize_with = "lenient")]
    pub notifications: Notifications,
    /// How the sidebar lists sessions.
    #[serde(deserialize_with = "lenient")]
    pub sidebar: SidebarView,
    /// Header/footer order and visibility, independently of plugin availability.
    #[serde(
        deserialize_with = "lenient",
        skip_serializing_if = "crate::sidebar::navigation::Navigation::is_empty"
    )]
    pub sidebar_navigation: crate::sidebar::navigation::Navigation,
    /// What new sessions start with, by machine.
    #[serde(deserialize_with = "lenient_entries")]
    pub defaults: BTreeMap<NodeId, MachineDefaults>,
}

/// Which panels show.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    /// The sidebar.
    #[serde(deserialize_with = "lenient")]
    pub sidebar: Panel,
    /// The inspector (a session's summary).
    #[serde(deserialize_with = "lenient")]
    pub inspector: Panel,
    /// The sidebar's width when dragged, whole points (`None`: the
    /// default, `panels::SIDEBAR`). Not written while `None`, so the
    /// document the page and the bridge share is unchanged until a panel
    /// is dragged.
    #[serde(deserialize_with = "lenient", skip_serializing_if = "Option::is_none")]
    pub sidebar_width: Option<u16>,
    /// The code panel's width when dragged (`panels::CODE_PANEL`).
    #[serde(deserialize_with = "lenient", skip_serializing_if = "Option::is_none")]
    pub panel_width: Option<u16>,
}

/// Whether a panel shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[serde(default)]
pub struct Composer {
    /// Which key sends.
    #[serde(deserialize_with = "lenient")]
    pub send: SendKey,
    /// What Send does while a turn runs.
    #[serde(deserialize_with = "lenient")]
    pub delivery: TurnDelivery,
}

/// Which key sends a prompt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendKey {
    /// Enter sends; Shift+Enter starts a line.
    #[default]
    Enter,
    /// Mod+Enter sends; Enter starts a line.
    ModEnter,
}

impl SendKey {
    /// Both, in the order Settings offers them.
    pub const ALL: [Self; 2] = [Self::Enter, Self::ModEnter];
}

/// What Send does with a prompt while the agent's turn runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnDelivery {
    /// It waits in the session's queue for the turn to end (behind a
    /// permission request it interrupts: `session::send_delivery`).
    Queue,
    /// It joins the running turn, where the agent can steer; otherwise it
    /// replaces the turn. Paseo's default (`storage.ts`, the owner's word
    /// 45).
    #[default]
    Steer,
    /// The turn is stopped and it starts the next.
    Interrupt,
}

impl TurnDelivery {
    /// Every way, in the order Settings offers them.
    pub const ALL: [Self; 3] = [Self::Queue, Self::Steer, Self::Interrupt];
}

/// How sessions are announced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    /// How a session that needs the person, or finished, is announced.
    #[serde(deserialize_with = "lenient")]
    pub alerts: Alerts,
    /// A count of notices in the window's title.
    #[serde(deserialize_with = "lenient_true")]
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

/// `title`'s default is on, which `bool`'s is not.
fn lenient_true<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    let read: Option<bool> = lenient(deserializer)?;
    Ok(read.unwrap_or(true))
}

/// How a notice is announced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarView {
    /// What sessions are grouped by.
    #[serde(deserialize_with = "lenient")]
    pub group: Grouping,
    /// Their order within a group.
    #[serde(deserialize_with = "lenient")]
    pub sort: SessionSort,
    /// Whether ended sessions are listed (on unless the person turned it
    /// off; see [`SidebarView::default`]).
    #[serde(deserialize_with = "lenient_on")]
    pub ended: bool,
}

/// [Decision Log] Ended sessions are listed by default (2026-11-14, the
/// coordinator's ruling after the 0.0.0.0.17 smoke test)
/// - Purpose: a machine whose sessions have all ended (interrupted or
///   exited) showed an empty sidebar after the update, though 0.0.0.0.16
///   listed them; the person's one session vanished until the switch was
///   found.
/// - Reference: Paseo lists a session until it is archived; the switch
///   stays for hiding ended ones.
/// - Before: the page's and the bridge's shared document (ADR 0036) says
///   `"ended": false` (`web/src/data/prefs-change.ts`,
///   `crates/vessel-web/src/wire/prefs.rs`), and this model followed it.
/// - Choice: native moves first: a document without the field reads as
///   on, and a stored `false` (the person's choice) stays off. The page
///   and the bridge are the node line's to move (its own slice); until
///   then the two read a document without the field differently, which
///   `tests/prefs.rs` pins as the one stated difference. Once they move,
///   the contract fixture says `true` and that exception goes.
impl Default for SidebarView {
    fn default() -> Self {
        Self {
            group: Grouping::default(),
            sort: SessionSort::default(),
            ended: true,
        }
    }
}

/// [`lenient`] for a switch that is on unless written off: an unreadable
/// value reads as on, as a missing one does.
fn lenient_on<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Maybe::<bool>::deserialize(deserializer)?
        .known()
        .unwrap_or(true))
}

/// What the sidebar groups sessions by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[serde(rename_all = "snake_case")]
pub enum SessionSort {
    /// Newest first, by creation.
    #[default]
    Created,
    /// Most recently active first.
    Activity,
}

/// What a new session on one machine starts with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MachineDefaults {
    /// The agent picked first.
    pub agent: Option<HarnessKind>,
    /// What each agent starts with.
    pub agents: BTreeMap<AgentKind, AgentDefaults>,
}

/// One of the four agents as a key (ordered by its wire name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentKind(pub HarnessKind);

impl Ord for AgentKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.name().cmp(other.0.name())
    }
}

impl PartialOrd for AgentKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// What one agent starts with on one machine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentDefaults {
    /// The profile; `None` is the agent's default.
    pub profile: Option<ProfileName>,
    /// The agent's own options and their values (at most
    /// [`MAX_CHOICES`], each id and value at most [`MAX_CHOICE_TEXT`]).
    pub choices: Choices,
}

/// An agent's kept choices, bounded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "BTreeMap<OptionId, ChoiceValue>",
    into = "BTreeMap<OptionId, ChoiceValue>"
)]
pub struct Choices(BTreeMap<OptionId, ChoiceValue>);

impl Choices {
    /// The choices.
    #[must_use]
    pub const fn get(&self) -> &BTreeMap<OptionId, ChoiceValue> {
        &self.0
    }
}

impl TryFrom<BTreeMap<OptionId, ChoiceValue>> for Choices {
    type Error = TooMany;

    fn try_from(choices: BTreeMap<OptionId, ChoiceValue>) -> Result<Self, TooMany> {
        let fits = |(id, value): (&OptionId, &ChoiceValue)| {
            id.as_str().len() <= MAX_CHOICE_TEXT
                && match value {
                    ChoiceValue::Select { value, .. } => value.as_str().len() <= MAX_CHOICE_TEXT,
                    ChoiceValue::Toggle { .. } => true,
                }
        };
        if choices.len() <= MAX_CHOICES && choices.iter().all(fits) {
            Ok(Self(choices))
        } else {
            Err(TooMany { max: MAX_CHOICES })
        }
    }
}

impl From<Choices> for BTreeMap<OptionId, ChoiceValue> {
    fn from(choices: Choices) -> Self {
        choices.0
    }
}

/// Where the preferences are kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kept {
    /// In a file, read at start and written after every change.
    File {
        /// The file, as display text.
        path: String,
    },
    /// In the app's memory, until it quits.
    ThisLaunch {
        /// Why not in a file.
        why: LaunchOnly,
    },
}

/// Before they are read from a place, they are kept for this launch: a
/// default that was never read is never written over a file.
impl Default for Kept {
    fn default() -> Self {
        Self::ThisLaunch {
            why: LaunchOnly::NoPlace,
        }
    }
}

/// Why the preferences are kept for this launch only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaunchOnly {
    /// This surface has no place to keep them (a browser, for now).
    NoPlace,
    /// Where to keep them could not be worked out.
    NoPath {
        /// Why.
        message: String,
    },
    /// The file could not be read; it is not touched in this launch.
    /// Where and what, never the file's text (`crate::files`).
    Unreadable {
        /// The file, where and what.
        fault: crate::files::FileFault,
    },
    /// The file was written by a newer Vessel; it is left as it is.
    Newer {
        /// The file, as display text.
        path: String,
        /// The version it says.
        version: u32,
    },
}
