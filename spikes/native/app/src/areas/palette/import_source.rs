//! Root import reuses W2's modal and the shared source lifecycle. This module
//! neither selects a host nor submits an import; the existing owner does that.
use super::source::Command;
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::Context;
use vessel_app_view::palette::PaletteGroup;
use vessel_app_view::palette::registration::{Contribution, Presentation, Rank, Visibility};

pub(super) fn install(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    if let Some(owner) = shell.areas.palette.root_import.take() {
        shell.remove_command_source(&owner, cx);
    }
    if !shell.areas.palette.is_open() {
        return;
    }
    let (Ok(source), Some(entry)) = ("native-import".parse(), entry(shell)) else {
        tracing::warn!("import source identity refused");
        return;
    };
    match shell.register_command_source(source, vec![entry], cx) {
        Ok(owner) => shell.areas.palette.root_import = Some(owner),
        Err(error) => tracing::warn!(%error, "import command source refused"),
    }
}

fn entry(shell: &Shell) -> Option<Contribution<Command>> {
    Some(Contribution {
        id: "import-session".parse().ok()?,
        group: "native-Actions".parse().ok()?,
        group_rank: Rank::new(0),
        section_title: Some(
            shell
                .t
                .lang
                .palette()
                .group(PaletteGroup::Actions)
                .to_owned(),
        ),
        rank: Rank::new(2),
        keywords: ["import", "session", "terminal"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        visibility: Visibility::Always,
        presentation: Presentation::Action {
            title: shell.words.history().title().to_owned(),
            subtitle: None,
        },
        target: Command::callback(Some(Glyph::Import), None, |shell, window, cx| {
            // The existing callback path has already closed the palette.
            shell.open_import(window, cx);
            true // The owner keeps its own refusal/modal feedback.
        }),
    })
}
