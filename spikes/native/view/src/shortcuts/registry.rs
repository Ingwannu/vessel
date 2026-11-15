//! The shortcut registry (the page's `data/shortcuts/registry.ts`): every
//! action a key can run, its group in Settings › Shortcuts, its default
//! keys, and the person's overrides.
//!
//! [Decision Log] One registry, the page's actions and keys (milestone 11)
//! - Purpose: the listener, the palette's hints, the sidebar's hints and
//!   Settings › Shortcuts read the same table, so a key shown is the key
//!   that works.
//! - Choice: the page's fourteen actions in its order with its defaults,
//!   and GOAT's two queue actions (send the first queued message now,
//!   edit the last one) after the composer's focus;
//!   an action has at most two bindings; "session 1–9" is fixed (Alt and
//!   a digit, one per session) and cannot be reassigned. An override
//!   replaces an action's defaults whole (an empty list unbinds it); ids
//!   the registry does not know are kept, so an older or newer build's
//!   overrides survive a round trip.
//! - The overrides are the preferences' (`Preferences::shortcuts`, track
//!   E's one model, the bridge's document): read here, changed only
//!   through a `PreferenceChange`.

use std::collections::BTreeMap;

use super::keys::{Binding, Chord};
use crate::prefs::{ActionId, Bindings, MAX_BINDINGS};

/// What a shortcut can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    /// Open the command palette.
    Palette,
    /// Start a new session.
    NewSession,
    /// Open Settings.
    Settings,
    /// Go to the board.
    GoBoard,
    /// Go to Settings › Machines.
    GoMachines,
    /// Go to Settings › Sync.
    GoSync,
    /// Go to Settings › Devices.
    GoDevices,
    /// The session above the open one in the sidebar's order.
    PreviousSession,
    /// The session below it.
    NextSession,
    /// The sidebar's session 1 to 9 (fixed keys).
    JumpSession,
    /// Find in the open session.
    Find,
    /// Focus the composer.
    FocusComposer,
    /// Send the first queued message now (GOAT's
    /// `thread.steerQueuedMessage`): into the running turn when the agent
    /// takes a steer, else as a turn of its own.
    SteerQueued,
    /// Edit the last queued message (GOAT's `thread.editQueuedMessage`),
    /// from the composer's very start only.
    EditQueued,
    /// Show or hide the sidebar.
    ToggleSidebar,
    /// Show or hide the session's summary.
    ToggleInspector,
}

/// The sections of Settings › Shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutGroup {
    /// The palette, a new session, Settings.
    General,
    /// Places and sessions.
    Navigation,
    /// Inside a session.
    Session,
    /// The sidebar and the summary.
    Panels,
}

impl ShortcutGroup {
    /// Every group, in Settings' order.
    pub const ALL: [Self; 4] = [Self::General, Self::Navigation, Self::Session, Self::Panels];
}

impl Action {
    /// Every action, in the registry's order.
    pub const ALL: [Self; 16] = [
        Self::Palette,
        Self::NewSession,
        Self::Settings,
        Self::GoBoard,
        Self::GoMachines,
        Self::GoSync,
        Self::GoDevices,
        Self::PreviousSession,
        Self::NextSession,
        Self::JumpSession,
        Self::Find,
        Self::FocusComposer,
        Self::SteerQueued,
        Self::EditQueued,
        Self::ToggleSidebar,
        Self::ToggleInspector,
    ];

    /// The id an override is kept under (the page's).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Palette => "palette",
            Self::NewSession => "new-session",
            Self::Settings => "settings",
            Self::GoBoard => "go-board",
            Self::GoMachines => "go-machines",
            Self::GoSync => "go-sync",
            Self::GoDevices => "go-devices",
            Self::PreviousSession => "previous-session",
            Self::NextSession => "next-session",
            Self::JumpSession => "jump-session",
            Self::Find => "find",
            Self::FocusComposer => "focus-composer",
            Self::SteerQueued => "steer-queued",
            Self::EditQueued => "edit-queued",
            Self::ToggleSidebar => "toggle-sidebar",
            Self::ToggleInspector => "toggle-inspector",
        }
    }

    /// Its id as the preferences keep it (every registry id is one; a
    /// test says so).
    #[must_use]
    pub fn action_id(self) -> Option<ActionId> {
        self.id().parse().ok()
    }

    /// The action an id names, if the registry knows it.
    #[must_use]
    pub fn of_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }

    /// Its section in Settings › Shortcuts.
    #[must_use]
    pub const fn group(self) -> ShortcutGroup {
        match self {
            Self::Palette | Self::NewSession | Self::Settings => ShortcutGroup::General,
            Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices
            | Self::PreviousSession
            | Self::NextSession
            | Self::JumpSession => ShortcutGroup::Navigation,
            Self::Find | Self::FocusComposer | Self::SteerQueued | Self::EditQueued => {
                ShortcutGroup::Session
            }
            Self::ToggleSidebar | Self::ToggleInspector => ShortcutGroup::Panels,
        }
    }

    /// Its keys before the person changes them, as an override writes
    /// them.
    #[must_use]
    pub const fn defaults(self) -> &'static [&'static str] {
        match self {
            Self::Palette => &["Mod+K", "/"],
            Self::NewSession => &["N"],
            Self::Settings => &["Mod+,"],
            Self::GoBoard => &["G S"],
            Self::GoMachines => &["G M", "G F"],
            Self::GoSync => &["G Y"],
            Self::GoDevices => &["G D"],
            Self::PreviousSession => &["Alt+["],
            Self::NextSession => &["Alt+]"],
            Self::JumpSession => &[],
            Self::Find => &["Mod+F"],
            Self::FocusComposer => &["Mod+L"],
            // GOAT's defaults (`packages/shared/src/keybindings.ts`).
            Self::SteerQueued => &["Mod+Shift+Enter"],
            Self::EditQueued => &["Alt+Up"],
            Self::ToggleSidebar => &["Mod+B"],
            Self::ToggleInspector => &["Mod+Shift+B"],
        }
    }

    /// Whether its keys are fixed: session 1 to 9 is Alt and a digit, one
    /// key per session.
    #[must_use]
    pub const fn fixed(self) -> bool {
        matches!(self, Self::JumpSession)
    }

    /// Every action of a group, in the registry's order.
    pub fn in_group(group: ShortcutGroup) -> impl Iterator<Item = Self> {
        Self::ALL
            .into_iter()
            .filter(move |action| action.group() == group)
    }
}

/// The session a chord jumps to: Alt alone with a digit from 1 to 9.
#[must_use]
pub fn jump_of(chord: &Chord) -> Option<u8> {
    if chord.primary || chord.ctrl || chord.shift || chord.meta || !chord.alt {
        return None;
    }
    let mut chars = chord.key.chars();
    let digit = chars.next()?.to_digit(10)?;
    if chars.next().is_some() || digit == 0 {
        return None;
    }
    u8::try_from(digit).ok()
}

/// The fixed keys of session 1 to 9, as a chord to show ("⌥1–9").
#[must_use]
pub fn jump_shown() -> Chord {
    Chord {
        primary: false,
        ctrl: false,
        alt: true,
        shift: false,
        meta: false,
        key: "1–9".to_owned(),
    }
}

/// Whether the person changed an action's keys (`overrides`: the
/// preferences' shortcuts, an action listed replacing its defaults whole,
/// an empty list unbinding it).
#[must_use]
pub fn modified(action: Action, overrides: &BTreeMap<ActionId, Bindings>) -> bool {
    overrides.keys().any(|id| id.as_str() == action.id())
}

/// An action's bindings: the person's, else its defaults; at most
/// [`MAX_BINDINGS`], any that does not parse left out.
#[must_use]
pub fn bindings_of(action: Action, overrides: &BTreeMap<ActionId, Bindings>) -> Vec<Binding> {
    overrides
        .iter()
        .find(|(id, _)| id.as_str() == action.id())
        .map_or_else(
            || {
                action
                    .defaults()
                    .iter()
                    .take(MAX_BINDINGS)
                    .filter_map(|text| Binding::parse(text))
                    .collect()
            },
            |(_, keys)| {
                keys.keys()
                    .iter()
                    .take(MAX_BINDINGS)
                    .filter_map(|key| Binding::parse(key.as_str()))
                    .collect()
            },
        )
}
