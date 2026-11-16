//! Features contribute typed closures to the existing command center. Captured
//! node/workspace/plugin addresses stay with their owner; the center never
//! interprets RPC names or manufactures a target from a displayed string.

use super::Target;
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::palette::registration::{
    Contribution, GroupId, Owner, RegistrationError, SourceId,
};
use vessel_app_view::shortcuts::Action;

type Run = dyn Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) -> bool;

#[derive(Clone)]
pub(crate) enum CommandAction {
    Callback(Rc<Run>),
    Page(GroupId),
    Legacy(Target),
}

#[derive(Clone)]
pub(crate) struct Command {
    pub(crate) action: CommandAction,
    pub(crate) glyph: Option<Glyph>,
    pub(crate) shortcut: Option<Action>,
}

impl Command {
    /// The callback must revalidate the captured resource generation/host before
    /// invoking the owner's existing operation. False leaves a safe unavailable
    /// notice; owner-specific errors remain the owner's responsibility.
    pub(crate) fn callback(
        glyph: Option<Glyph>,
        shortcut: Option<Action>,
        run: impl Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) -> bool + 'static,
    ) -> Self {
        Self {
            action: CommandAction::Callback(Rc::new(run)),
            glyph,
            shortcut,
        }
    }
    pub(crate) const fn page(group: GroupId) -> Self {
        Self {
            action: CommandAction::Page(group),
            glyph: None,
            shortcut: None,
        }
    }
}

impl Shell {
    /// A source atomically replaces its own translated projection. Duplicate ids
    /// or empty titles retain the previous set. Store the returned cleanup token.
    pub(crate) fn register_command_source(
        &mut self,
        source: SourceId,
        entries: Vec<Contribution<Command>>,
        cx: &mut Context<'_, Self>,
    ) -> Result<Owner, RegistrationError> {
        if source.as_str() == super::snapshot::ROOT_SOURCE {
            return Err(RegistrationError::InvalidId);
        }
        let owner = self.areas.palette.sources.replace(source, entries)?;
        cx.notify();
        Ok(owner)
    }
    pub(crate) fn remove_command_source(&mut self, owner: &Owner, cx: &mut Context<'_, Self>) {
        self.areas.palette.sources.remove(owner);
        cx.notify();
    }
}

/// Root help follows the same registration lifecycle that external feature
/// sources use. Its label is rebuilt only on open/language/availability events.
pub(super) fn install_help(shell: &mut Shell, _window: &Window, cx: &mut Context<'_, Shell>) {
    if let Some(owner) = shell.areas.palette.root_help.take() {
        shell.remove_command_source(&owner, cx);
    }
    if !shell.areas.palette.is_open() {
        return;
    }
    let (Ok(source), Ok(id), Ok(group)) = (
        "native-help".parse(),
        "keyboard-shortcuts".parse(),
        "native-Actions".parse(),
    ) else {
        return;
    };
    let entry = Contribution {
        id,
        group,
        group_rank: vessel_app_view::palette::registration::Rank::new(0),
        section_title: Some(
            shell
                .t
                .lang
                .palette()
                .group(vessel_app_view::palette::PaletteGroup::Actions)
                .to_owned(),
        ),
        rank: vessel_app_view::palette::registration::Rank::new(7),
        keywords: ["keyboard", "shortcuts", "keys", "hotkeys"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        visibility: vessel_app_view::palette::registration::Visibility::Always,
        presentation: vessel_app_view::palette::registration::Presentation::Action {
            title: vessel_app_view::shortcuts::action_label(Action::Help, shell.words).to_owned(),
            subtitle: None,
        },
        target: Command::callback(
            Some(Glyph::Keyboard),
            Some(Action::Help),
            |shell, window, cx| {
                if !crate::areas::shortcuts::available(window) {
                    return false;
                }
                shell.open_shortcuts_help(window, cx);
                true
            },
        ),
    };
    match shell.register_command_source(source, vec![entry], cx) {
        Ok(owner) => shell.areas.palette.root_help = Some(owner),
        Err(error) => tracing::warn!(%error, "command source registration refused"),
    }
}

/// Reuse feature projections; every key/click rechecks the actual live window.
pub(super) fn refresh_dynamic(shell: &mut Shell, window: &Window) {
    #[cfg(test)]
    super::git_source::refresh(shell);
    super::layout_source::refresh(shell, window);
}
