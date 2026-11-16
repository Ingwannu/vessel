//! Root/native contributions reuse the palette's previous actions and captions.
use super::source::{Command, CommandAction};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_view::palette::registration::{
    ChoicePath, Contribution, Presentation, Rank, Selection, Visibility,
};
use vessel_app_view::palette::{PaletteGroup, PaletteTarget};
use vessel_app_view::shortcuts::Action;

struct Policy {
    id: String,
    group: PaletteGroup,
    rank: i16,
    visibility: Visibility,
}

pub(super) fn build(shell: &Shell, all: &[super::Entry]) -> Vec<Contribution<Command>> {
    let mut entries: Vec<_> = all
        .iter()
        .enumerate()
        .filter_map(|(at, entry)| contribution(shell, entry, at))
        .collect();
    entries.extend(extra(shell));
    entries
}

fn policy(entry: &super::Entry, at: usize) -> Option<Policy> {
    if entry.group == PaletteGroup::Sessions {
        return None;
    }
    let (id, group, rank, visibility) = match entry.target {
        // A1 contributes its admitted opener. A legacy per-machine creation
        // row cannot stand in for the canonical workspace destination.
        PaletteTarget::NewSession(_) => return None,
        PaletteTarget::Settings(vessel_app_view::settings::Section::General) => (
            "settings".to_owned(),
            PaletteGroup::Actions,
            6,
            Visibility::Always,
        ),
        // Legacy destinations remain searchable extensions. Their existence
        // never manufactures the still-unready Home/History/Schedules routes.
        PaletteTarget::Messages
        | PaletteTarget::AddMachine
        | PaletteTarget::Board
        | PaletteTarget::Settings(_)
        | PaletteTarget::Machine(_)
        | PaletteTarget::Session { .. }
        | PaletteTarget::Theme(_) => (
            entry.id.clone(),
            entry.group,
            i16::try_from(at).ok()?,
            Visibility::Query,
        ),
    };
    Some(Policy {
        id,
        group,
        rank,
        visibility,
    })
}

fn presentation(shell: &Shell, entry: &super::Entry) -> Option<Presentation> {
    if let PaletteTarget::Theme(theme) = entry.target {
        return Some(Presentation::Choice {
            path: ChoicePath::new(
                shell.t.lang.palette().group(entry.group).to_owned(),
                vec![entry.label.clone()],
            )
            .ok()?,
            selection: if theme == shell.prefs.settings.appearance.theme {
                Selection::Selected
            } else {
                Selection::Unselected
            },
        });
    }
    let title = match entry.target {
        PaletteTarget::NewSession(_) => return None,
        PaletteTarget::Settings(vessel_app_view::settings::Section::General) => {
            vessel_app_view::shortcuts::action_label(Action::Settings, shell.words).to_owned()
        }
        PaletteTarget::Messages
        | PaletteTarget::AddMachine
        | PaletteTarget::Board
        | PaletteTarget::Settings(_)
        | PaletteTarget::Machine(_)
        | PaletteTarget::Session { .. }
        | PaletteTarget::Theme(_) => entry.label.clone(),
    };
    Some(Presentation::Action {
        title,
        subtitle: entry
            .detail
            .as_ref()
            .map(|detail| detail.search_text().into_owned()),
    })
}

fn contribution(shell: &Shell, entry: &super::Entry, at: usize) -> Option<Contribution<Command>> {
    let policy = policy(entry, at)?;
    let settings = matches!(
        entry.target,
        PaletteTarget::Settings(vessel_app_view::settings::Section::General)
    );
    Some(Contribution {
        id: policy.id.parse().ok()?,
        group: format!("native-{:?}", policy.group).parse().ok()?,
        group_rank: Rank::new(rank_of(policy.group)),
        section_title: Some(shell.t.lang.palette().group(policy.group).to_owned()),
        rank: Rank::new(policy.rank),
        keywords: if settings {
            ["settings", "preferences", "config", "configuration"]
                .into_iter()
                .map(str::to_owned)
                .collect()
        } else {
            entry.keywords.iter().cloned().collect()
        },
        visibility: policy.visibility,
        presentation: presentation(shell, entry)?,
        target: if settings {
            Command::callback(
                Some(super::rows::glyph_of(&entry.target)),
                Some(Action::Settings),
                |shell, window, cx| {
                    shell.open_settings(None, window, cx);
                    true
                },
            )
        } else {
            Command {
                glyph: Some(super::rows::glyph_of(&entry.target)),
                shortcut: super::rows::action_of(&entry.target),
                action: CommandAction::Legacy(entry.target.clone()),
            }
        },
    })
}

fn extra(shell: &Shell) -> Vec<Contribution<Command>> {
    let mut entries = Vec::new();
    entries.extend(left_sidebar(shell));
    if let (Ok(id), Ok(group)) = ("search-messages".parse(), "native-Actions".parse()) {
        entries.push(Contribution {
            id,
            group,
            group_rank: Rank::new(0),
            section_title: Some(
                shell
                    .t
                    .lang
                    .palette()
                    .group(PaletteGroup::Actions)
                    .to_owned(),
            ),
            rank: Rank::new(90),
            keywords: vec!["messages search".to_owned()],
            visibility: Visibility::Always,
            presentation: Presentation::Action {
                title: shell.words.message_search().open().to_owned(),
                subtitle: None,
            },
            target: Command {
                action: CommandAction::Legacy(PaletteTarget::Messages),
                glyph: Some(Glyph::Message),
                shortcut: None,
            },
        });
    }
    if let (Ok(id), Ok(group), Ok(page)) = (
        "theme-choices".parse(),
        "native-Actions".parse(),
        "native-Theme".parse(),
    ) {
        entries.push(Contribution {
            id,
            group,
            group_rank: Rank::new(0),
            section_title: Some(
                shell
                    .t
                    .lang
                    .palette()
                    .group(PaletteGroup::Actions)
                    .to_owned(),
            ),
            rank: Rank::new(9),
            keywords: vec!["theme appearance".to_owned()],
            visibility: Visibility::Always,
            presentation: Presentation::Action {
                title: shell.t.lang.palette().group(PaletteGroup::Theme).to_owned(),
                subtitle: None,
            },
            target: Command::page(page),
        });
    }
    entries
}

fn left_sidebar(shell: &Shell) -> Option<Contribution<Command>> {
    Some(Contribution {
        id: "toggle-left-sidebar".parse().ok()?,
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
        rank: Rank::new(8),
        keywords: ["toggle", "sidebar", "left", "panel", "workspaces"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        visibility: Visibility::Query,
        presentation: Presentation::Action {
            title: vessel_app_view::shortcuts::action_label(Action::ToggleSidebar, shell.words)
                .to_owned(),
            subtitle: None,
        },
        target: Command::callback(
            Some(Glyph::PanelLeft),
            Some(Action::ToggleSidebar),
            Shell::dispatch_sidebar_toggle,
        ),
    })
}
const fn rank_of(group: PaletteGroup) -> i16 {
    match group {
        PaletteGroup::Actions => 0,
        PaletteGroup::GoTo => 10,
        PaletteGroup::Machines => 20,
        PaletteGroup::Sessions => 30,
        PaletteGroup::Theme => 40,
    }
}
