//! The "Terminal profiles" group of the New tab menus: the pane header's
//! "+" (Paseo's `WorkspaceNewTabMenuContent`) and, on a compact window, the
//! workspace "…" (Paseo's `WorkspaceHeaderMenuMobile`).
//!
//! [Decision Log]
//! - Reference: after a separator, the group's label, then one row per
//!   profile in the list's order led by its mark (14 in "+", 16 in "…";
//!   the terminal glyph when it has none, the bot for an icon outside the
//!   catalogue), then a separator and a plain "Edit profiles" row, which
//!   opens Settings › Terminals for the workspace's machine. A row waits
//!   while a terminal is being created (Paseo's `createTerminalDisabled`).
//! - Choice: one builder for both menus, over the list New tab's panel
//!   reads (`terminal_profiles_shown`), with the kit menu's label row and
//!   mark leading; nothing while the list is unread or empty.
//! - Difference: Paseo's compact menu lists its default profiles before
//!   the host's config arrives; Vessel lists none until the node answers,
//!   so no row is offered that the node would not run.

use vessel_app_kit::MenuItem;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::menu::LeadingSize;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::agent_mark::AgentMark;
use vessel_app_view::layout::TabId;
use vessel_app_view::settings::Section;
use vessel_proto::terminal_profiles::Id;

use crate::shell::Shell;

/// What a row of the group does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Choice {
    /// Open a terminal running this profile.
    Launch(Id),
    /// Open Settings › Terminals for the workspace's machine.
    Edit,
}

/// The group's rows and what each does (`None`: its label), or nothing.
/// `size`: the menu's leading size (Paseo's 14 in "+", 16 in "…").
pub(crate) fn group(shell: &Shell, size: LeadingSize) -> Vec<(Option<Choice>, MenuItem)> {
    let Some(list) = shell
        .terminal_profiles_shown()
        .filter(|list| !list.entries().is_empty())
    else {
        return Vec::new();
    };
    let words = shell.words.layout();
    let waiting = shell.terminal_tab_creating();
    let mut rows = vec![(
        None,
        MenuItem::heading(words.terminal_profiles().to_owned()).grouped(),
    )];
    rows.extend(list.entries().iter().map(|profile| {
        let item = MenuItem::new(profile.name().as_str().to_owned());
        let item = match AgentMark::of_profile(profile) {
            None => item.with_leading_at(Glyph::SquareTerminal, size),
            Some(mark) => match crate::agent_mark::artwork(mark) {
                Some(artwork) => item.with_mark(artwork, size),
                None => item.with_leading_at(Glyph::Bot, size),
            },
        };
        let item = MenuItem {
            disabled: waiting,
            ..item
        };
        (Some(Choice::Launch(profile.id().clone())), item)
    }));
    rows.push((
        Some(Choice::Edit),
        MenuItem::new(words.edit_profiles().to_owned()).grouped(),
    ));
    rows
}

impl Shell {
    /// A row of the group was chosen.
    pub(crate) fn terminal_profiles_choose(
        &mut self,
        choice: Choice,
        replaces: Option<TabId>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        match choice {
            Choice::Launch(id) => self.terminal_tab_new_profile(id, replaces, window, cx),
            Choice::Edit => {
                if let Some(machine) = super::workspace_machine(self) {
                    self.areas.settings.machine = Some(machine);
                }
                self.open_settings(Some(Section::Terminals), window, cx);
            }
        }
    }
}
