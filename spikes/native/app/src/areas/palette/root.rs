//! Root/native contributions reuse the palette's previous actions and captions.
use super::source::{Command, CommandAction};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_view::palette::registration::{
    ChoicePath, Contribution, Presentation, Rank, Selection, Visibility,
};
use vessel_app_view::palette::{PaletteGroup, PaletteTarget};
use vessel_app_view::shortcuts::Action;

pub(super) fn build(shell: &Shell, all: &[super::Entry]) -> Vec<Contribution<Command>> {
    let mut entries = Vec::new();
    for (at, entry) in all
        .iter()
        .filter(|entry| entry.group != PaletteGroup::Sessions)
        .enumerate()
    {
        let (Ok(id), Ok(group)) = (
            entry.id.parse(),
            format!("native-{:?}", entry.group).parse(),
        ) else {
            continue;
        };
        let Ok(rank) = i16::try_from(at) else {
            continue;
        };
        let presentation = match entry.target {
            PaletteTarget::Theme(theme) => {
                let Ok(path) = ChoicePath::new(
                    shell.t.lang.palette().group(entry.group).to_owned(),
                    vec![entry.label.clone()],
                ) else {
                    continue;
                };
                Presentation::Choice {
                    path,
                    selection: if theme == shell.prefs.settings.appearance.theme {
                        Selection::Selected
                    } else {
                        Selection::Unselected
                    },
                }
            }
            PaletteTarget::Messages
            | PaletteTarget::NewSession(_)
            | PaletteTarget::AddMachine
            | PaletteTarget::Board
            | PaletteTarget::Settings(_)
            | PaletteTarget::Machine(_)
            | PaletteTarget::Session { .. } => Presentation::Action {
                title: if matches!(entry.target, PaletteTarget::NewSession(_)) {
                    vessel_app_view::shortcuts::action_label(Action::NewSession, shell.words)
                        .to_owned()
                } else {
                    entry.label.clone()
                },
                subtitle: entry
                    .detail
                    .as_ref()
                    .map(|detail| detail.search_text().into_owned()),
            },
        };
        entries.push(Contribution {
            id,
            group,
            group_rank: Rank::new(rank_of(entry.group)),
            section_title: Some(shell.t.lang.palette().group(entry.group).to_owned()),
            rank: Rank::new(rank),
            keywords: entry.keywords.iter().cloned().collect(),
            visibility: if entry.group == PaletteGroup::Theme {
                Visibility::Query
            } else {
                Visibility::Always
            },
            presentation,
            target: if matches!(entry.target, PaletteTarget::NewSession(_)) {
                Command::shortcut(
                    Action::NewSession,
                    Some(super::rows::glyph_of(&entry.target)),
                )
            } else {
                Command {
                    glyph: Some(super::rows::glyph_of(&entry.target)),
                    shortcut: super::rows::action_of(&entry.target),
                    action: CommandAction::Legacy(entry.target.clone()),
                }
            },
        });
    }
    entries.extend(extra(shell));
    entries
}

fn extra(shell: &Shell) -> Vec<Contribution<Command>> {
    let mut entries = Vec::new();
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
const fn rank_of(group: PaletteGroup) -> i16 {
    match group {
        PaletteGroup::Actions => 0,
        PaletteGroup::GoTo => 10,
        PaletteGroup::Machines => 20,
        PaletteGroup::Sessions => 30,
        PaletteGroup::Theme => 40,
    }
}
