//! Stable source help ids and retained native action aliases.

use super::Action;
use crate::prefs::ActionId;

impl Action {
    /// Legacy action identity, retained for preference migration.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Palette => "palette",
            Self::SearchFiles => "search-files",
            Self::Help => "show-shortcuts",
            Self::Settings => "settings",
            Self::CycleTheme => "cycle-theme",
            Self::NewWindow => "new-window",
            Self::CloseWindow => "close-window",
            Self::Fullscreen => "fullscreen",
            Self::ZoomIn => "zoom-in",
            Self::ZoomOut => "zoom-out",
            Self::ActualSize => "actual-size",
            Self::OpenProject => "new-agent",
            Self::NewSession => "new-session",
            Self::JumpSession => "jump-session",
            Self::PreviousSession => "previous-session",
            Self::NextSession => "next-session",
            Self::PinWorkspace => "pin-workspace",
            Self::ArchiveWorkspace => "archive-workspace",
            Self::GoBoard => "go-board",
            Self::GoMachines => "go-machines",
            Self::GoSync => "go-sync",
            Self::GoDevices => "go-devices",
            Self::NewTab => "workspace-tab-new",
            Self::TabAgent => "workspace-tab-target-agent",
            Self::NewTerminal => "workspace-terminal-new",
            Self::TabBrowser => "workspace-tab-target-browser",
            Self::TabChanges => "workspace-tab-target-changes",
            Self::TabFiles => "workspace-tab-target-files",
            Self::CloseTab => "workspace-tab-close-current",
            Self::JumpTab => "workspace-tab-jump-index",
            Self::PreviousTab => "workspace-tab-prev",
            Self::NextTab => "workspace-tab-next",
            Self::SplitRight => "workspace-pane-split-right",
            Self::SplitDown => "workspace-pane-split-down",
            Self::FocusLeft => "workspace-pane-focus-left",
            Self::FocusRight => "workspace-pane-focus-right",
            Self::FocusUp => "workspace-pane-focus-up",
            Self::FocusDown => "workspace-pane-focus-down",
            Self::MoveTabLeft => "workspace-pane-move-tab-left",
            Self::MoveTabRight => "workspace-pane-move-tab-right",
            Self::MoveTabUp => "workspace-pane-move-tab-up",
            Self::MoveTabDown => "workspace-pane-move-tab-down",
            Self::ClosePane => "workspace-pane-close",
            Self::ToggleSidebar => "toggle-sidebar",
            Self::ToggleInspector => "toggle-inspector",
            Self::ToggleSidebars => "toggle-both-sidebars",
            Self::ToggleFocus => "toggle-focus",
            Self::FocusComposer => "focus-composer",
            Self::CycleMode => "cycle-agent-mode",
            Self::ToggleVoice => "voice-toggle",
            Self::ToggleDictation => "dictation-toggle",
            Self::Interrupt => "agent-interrupt",
            Self::MuteVoice => "voice-mute-toggle",
            Self::ConfirmDictation => "confirm-dictation",
            Self::Find => "find",
            Self::SteerQueued => "steer-queued",
            Self::EditQueued => "edit-queued",
        }
    }

    /// Reference help identity, independent of historical action aliases.
    #[must_use]
    pub const fn help_id(self) -> &'static str {
        match self {
            Self::Palette => "toggle-command-center",
            Self::SearchFiles => "search-files",
            Self::Help => "show-shortcuts",
            Self::Settings => "toggle-settings",
            Self::CycleTheme => "cycle-theme",
            Self::NewWindow => "new-window",
            Self::CloseWindow => "close-window",
            Self::Fullscreen => "fullscreen",
            Self::ZoomIn => "zoom-in",
            Self::ZoomOut => "zoom-out",
            Self::ActualSize => "actual-size",
            Self::OpenProject => "new-agent",
            Self::NewSession => "new-workspace",
            Self::JumpSession => "workspace-jump-index",
            Self::PreviousSession => "workspace-prev",
            Self::NextSession => "workspace-next",
            Self::PinWorkspace => "pin-workspace",
            Self::ArchiveWorkspace => "archive-workspace",
            Self::GoBoard => "go-board",
            Self::GoMachines => "go-machines",
            Self::GoSync => "go-sync",
            Self::GoDevices => "go-devices",
            Self::NewTab => "workspace-tab-new",
            Self::TabAgent => "workspace-tab-target-agent",
            Self::NewTerminal => "workspace-terminal-new",
            Self::TabBrowser => "workspace-tab-target-browser",
            Self::TabChanges => "workspace-tab-target-changes",
            Self::TabFiles => "workspace-tab-target-files",
            Self::CloseTab => "workspace-tab-close-current",
            Self::JumpTab => "workspace-tab-jump-index",
            Self::PreviousTab => "workspace-tab-prev",
            Self::NextTab => "workspace-tab-next",
            Self::SplitRight => "workspace-pane-split-right",
            Self::SplitDown => "workspace-pane-split-down",
            Self::FocusLeft => "workspace-pane-focus-left",
            Self::FocusRight => "workspace-pane-focus-right",
            Self::FocusUp => "workspace-pane-focus-up",
            Self::FocusDown => "workspace-pane-focus-down",
            Self::MoveTabLeft => "workspace-pane-move-tab-left",
            Self::MoveTabRight => "workspace-pane-move-tab-right",
            Self::MoveTabUp => "workspace-pane-move-tab-up",
            Self::MoveTabDown => "workspace-pane-move-tab-down",
            Self::ClosePane => "workspace-pane-close",
            Self::ToggleSidebar => "toggle-left-sidebar",
            Self::ToggleInspector => "toggle-right-sidebar",
            Self::ToggleSidebars => "toggle-both-sidebars",
            Self::ToggleFocus => "toggle-focus",
            Self::FocusComposer => "focus-message-input",
            Self::CycleMode => "cycle-agent-mode",
            Self::ToggleVoice => "voice-toggle",
            Self::ToggleDictation => "dictation-toggle",
            Self::Interrupt => "agent-interrupt",
            Self::MuteVoice => "voice-mute-toggle",
            Self::ConfirmDictation => "confirm-dictation",
            Self::Find => "find",
            Self::SteerQueued => "steer-queued",
            Self::EditQueued => "edit-queued",
        }
    }

    /// Parse once at the preference boundary.
    #[must_use]
    pub fn action_id(self) -> Option<ActionId> {
        self.id().parse().ok()
    }

    /// Resolve a historical id without string-based action dispatch.
    #[must_use]
    pub fn of_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }

    /// The previous table index, only for old table consumers during migration.
    #[must_use]
    pub const fn legacy_index(self) -> Option<usize> {
        match self {
            Self::Palette => Some(0),
            Self::NewSession => Some(1),
            Self::Settings => Some(2),
            Self::CycleTheme => Some(3),
            Self::GoBoard => Some(4),
            Self::GoMachines => Some(5),
            Self::GoSync => Some(6),
            Self::GoDevices => Some(7),
            Self::PreviousSession => Some(8),
            Self::NextSession => Some(9),
            Self::JumpSession => Some(10),
            Self::Find => Some(11),
            Self::FocusComposer => Some(12),
            Self::SteerQueued => Some(13),
            Self::EditQueued => Some(14),
            Self::ToggleSidebar => Some(15),
            Self::ToggleInspector => Some(16),
            Self::SearchFiles
            | Self::Help
            | Self::NewWindow
            | Self::CloseWindow
            | Self::Fullscreen
            | Self::ZoomIn
            | Self::ZoomOut
            | Self::ActualSize
            | Self::OpenProject
            | Self::PinWorkspace
            | Self::ArchiveWorkspace
            | Self::NewTab
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
            | Self::ClosePane
            | Self::ToggleSidebars
            | Self::ToggleFocus
            | Self::CycleMode
            | Self::ToggleVoice
            | Self::ToggleDictation
            | Self::Interrupt
            | Self::MuteVoice
            | Self::ConfirmDictation => None,
        }
    }
}
