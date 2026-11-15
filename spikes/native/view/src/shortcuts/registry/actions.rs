//! Closed shortcut actions. The default table is in the parent registry;
//! adding a variant forces every feature adapter and Fluent label match to care.

use super::{ShortcutGroup, ShortcutScope};
mod adapters;
mod identity;
mod layout;
use crate::shortcuts::keys::Platform;

/// What the one shortcut engine can ask a feature to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    /// Toggle command center.
    Palette,
    /// Search files.
    SearchFiles,
    /// Show keyboard shortcuts.
    Help,
    /// Toggle settings.
    Settings,
    /// Cycle theme.
    CycleTheme,
    /// New window.
    NewWindow,
    /// Close window.
    CloseWindow,
    /// Toggle fullscreen.
    Fullscreen,
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
    /// Actual size.
    ActualSize,
    /// Open project.
    OpenProject,
    /// New workspace.
    NewSession,
    /// Jump to workspace.
    JumpSession,
    /// Previous workspace.
    PreviousSession,
    /// Next workspace.
    NextSession,
    /// Pin chat.
    PinWorkspace,
    /// Archive workspace.
    ArchiveWorkspace,
    /// Go to the board.
    GoBoard,
    /// Go to Hosts.
    GoMachines,
    /// Go to Sync.
    GoSync,
    /// Go to Devices.
    GoDevices,
    /// New tab.
    NewTab,
    /// New agent.
    TabAgent,
    /// New terminal.
    NewTerminal,
    /// New browser.
    TabBrowser,
    /// Changes.
    TabChanges,
    /// Files.
    TabFiles,
    /// Close current tab.
    CloseTab,
    /// Jump to tab.
    JumpTab,
    /// Previous tab.
    PreviousTab,
    /// Next tab.
    NextTab,
    /// Split pane right.
    SplitRight,
    /// Split pane down.
    SplitDown,
    /// Focus pane left.
    FocusLeft,
    /// Focus pane right.
    FocusRight,
    /// Focus pane up.
    FocusUp,
    /// Focus pane down.
    FocusDown,
    /// Move tab left.
    MoveTabLeft,
    /// Move tab right.
    MoveTabRight,
    /// Move tab up.
    MoveTabUp,
    /// Move tab down.
    MoveTabDown,
    /// Close pane.
    ClosePane,
    /// Toggle left sidebar.
    ToggleSidebar,
    /// Toggle Explorer sidebar.
    ToggleInspector,
    /// Toggle both sidebars.
    ToggleSidebars,
    /// Toggle focus mode.
    ToggleFocus,
    /// Focus message input.
    FocusComposer,
    /// Cycle agent mode.
    CycleMode,
    /// Toggle voice mode.
    ToggleVoice,
    /// Start/stop dictation.
    ToggleDictation,
    /// Interrupt agent.
    Interrupt,
    /// Mute/unmute voice mode.
    MuteVoice,
    /// Confirm dictation.
    ConfirmDictation,
    /// Find in the session.
    Find,
    /// Send the first queued message now.
    SteerQueued,
    /// Edit the last queued message.
    EditQueued,
}

impl Action {
    /// Help order, also the default Settings row order.
    pub const ALL: [Self; 57] = [
        Self::Palette,
        Self::SearchFiles,
        Self::Help,
        Self::Settings,
        Self::CycleTheme,
        Self::NewWindow,
        Self::CloseWindow,
        Self::Fullscreen,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ActualSize,
        Self::OpenProject,
        Self::NewSession,
        Self::JumpSession,
        Self::PreviousSession,
        Self::NextSession,
        Self::PinWorkspace,
        Self::ArchiveWorkspace,
        Self::GoBoard,
        Self::GoMachines,
        Self::GoSync,
        Self::GoDevices,
        Self::NewTab,
        Self::TabAgent,
        Self::NewTerminal,
        Self::TabBrowser,
        Self::TabChanges,
        Self::TabFiles,
        Self::CloseTab,
        Self::JumpTab,
        Self::PreviousTab,
        Self::NextTab,
        Self::SplitRight,
        Self::SplitDown,
        Self::FocusLeft,
        Self::FocusRight,
        Self::FocusUp,
        Self::FocusDown,
        Self::MoveTabLeft,
        Self::MoveTabRight,
        Self::MoveTabUp,
        Self::MoveTabDown,
        Self::ClosePane,
        Self::ToggleSidebar,
        Self::ToggleInspector,
        Self::ToggleSidebars,
        Self::ToggleFocus,
        Self::FocusComposer,
        Self::CycleMode,
        Self::ToggleVoice,
        Self::ToggleDictation,
        Self::Interrupt,
        Self::MuteVoice,
        Self::ConfirmDictation,
        Self::Find,
        Self::SteerQueued,
        Self::EditQueued,
    ];

    /// Its translated section.
    #[must_use]
    pub const fn group(self) -> ShortcutGroup {
        match self {
            Self::Palette
            | Self::SearchFiles
            | Self::Help
            | Self::Settings
            | Self::CycleTheme
            | Self::NewWindow
            | Self::CloseWindow
            | Self::Fullscreen
            | Self::ZoomIn
            | Self::ZoomOut
            | Self::ActualSize => ShortcutGroup::General,
            Self::OpenProject
            | Self::NewSession
            | Self::JumpSession
            | Self::PreviousSession
            | Self::NextSession
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices => ShortcutGroup::Navigation,
            Self::NewTab
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::CloseTab
            | Self::JumpTab
            | Self::PreviousTab
            | Self::NextTab
            | Self::SplitRight
            | Self::SplitDown
            | Self::FocusLeft
            | Self::FocusRight
            | Self::FocusUp
            | Self::FocusDown
            | Self::MoveTabLeft
            | Self::MoveTabRight
            | Self::MoveTabUp
            | Self::MoveTabDown
            | Self::ClosePane => ShortcutGroup::TabsPanes,
            Self::ToggleSidebar
            | Self::ToggleInspector
            | Self::ToggleSidebars
            | Self::ToggleFocus => ShortcutGroup::Panels,
            Self::FocusComposer
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::Interrupt
            | Self::MuteVoice
            | Self::ConfirmDictation
            | Self::Find
            | Self::SteerQueued
            | Self::EditQueued => ShortcutGroup::Session,
        }
    }

    /// Payload shape is independent of the current shortcut key.
    #[must_use]
    pub const fn payload(self) -> crate::shortcuts::engine::Payload {
        match self {
            Self::JumpSession | Self::JumpTab => crate::shortcuts::engine::Payload::Index,
            Self::Palette
            | Self::SearchFiles
            | Self::Help
            | Self::Settings
            | Self::CycleTheme
            | Self::NewWindow
            | Self::CloseWindow
            | Self::Fullscreen
            | Self::ZoomIn
            | Self::ZoomOut
            | Self::ActualSize
            | Self::OpenProject
            | Self::NewSession
            | Self::PreviousSession
            | Self::NextSession
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::GoBoard
            | Self::GoMachines
            | Self::GoSync
            | Self::GoDevices
            | Self::NewTab
            | Self::TabAgent
            | Self::NewTerminal
            | Self::TabBrowser
            | Self::TabChanges
            | Self::TabFiles
            | Self::CloseTab
            | Self::PreviousTab
            | Self::NextTab
            | Self::SplitRight
            | Self::SplitDown
            | Self::FocusLeft
            | Self::FocusRight
            | Self::FocusUp
            | Self::FocusDown
            | Self::MoveTabLeft
            | Self::MoveTabRight
            | Self::MoveTabUp
            | Self::MoveTabDown
            | Self::ClosePane
            | Self::ToggleSidebar
            | Self::ToggleInspector
            | Self::ToggleSidebars
            | Self::ToggleFocus
            | Self::FocusComposer
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::Interrupt
            | Self::MuteVoice
            | Self::ConfirmDictation
            | Self::Find
            | Self::SteerQueued
            | Self::EditQueued => crate::shortcuts::engine::Payload::None,
        }
    }

    /// Current host defaults, derived from the one binding table.
    #[must_use]
    pub fn defaults(self) -> Vec<String> {
        super::default_combos(self, Platform::native())
    }

    /// Index bindings are now rebindable, like every other shortcut.
    #[must_use]
    pub const fn fixed(self) -> bool {
        false
    }

    /// Compatibility scope query delegates to the canonical binding predicates.
    #[must_use]
    pub fn enabled(self, scope: ShortcutScope, platform: Platform) -> bool {
        super::enabled(self, scope, platform)
    }

    /// Every action in a help section, retaining the reference order.
    pub fn in_group(group: ShortcutGroup) -> impl Iterator<Item = Self> {
        Self::ALL
            .into_iter()
            .filter(move |action| action.group() == group)
    }
}
