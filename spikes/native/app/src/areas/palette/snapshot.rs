//! One projection for pointer and keyboard selection. Contributions occupy the
//! first band; recent built-ins retain the view's matching/recency ordering.
use super::{
    Target,
    source::{Command, CommandAction},
};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_view::palette::pages::{ChoiceLocation, ChoiceRow, choice_rows_search};
use vessel_app_view::palette::registration::{
    ChoicePath, Contribution, Owner, Presentation, Rank, ResultId, Selection, Visibility,
};
use vessel_app_view::palette::{
    PaletteGroup, PaletteTarget, Subtitle, contribution_sections, palette_groups,
};
use vessel_app_view::shortcuts::Action;

pub(super) const ROOT_SOURCE: &str = "native-root";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Identity {
    Registered(ResultId),
    Legacy(String),
    Page(ChoiceLocation),
}
#[derive(Clone)]
pub(super) enum Activate {
    Registered(ResultId, Owner),
    Root(CommandAction),
    Legacy(Target),
    Page(ChoiceLocation),
}
pub(super) struct Row {
    pub(super) id: Identity,
    pub(super) title: String,
    pub(super) subtitle: Option<Subtitle>,
    pub(super) glyph: Option<Glyph>,
    pub(super) shortcut: Option<Action>,
    pub(super) selected: bool,
    pub(super) activate: Activate,
}
pub(super) struct Section {
    pub(super) title: Option<String>,
    pub(super) rows: Vec<Row>,
}
pub(super) struct Snapshot {
    pub(super) sections: Vec<Section>,
}

impl Snapshot {
    pub(super) fn ids(&self) -> Vec<&Identity> {
        self.sections
            .iter()
            .flat_map(|section| section.rows.iter().map(|row| &row.id))
            .collect()
    }
    pub(super) fn row(&self, id: &Identity) -> Option<&Row> {
        self.sections
            .iter()
            .flat_map(|section| &section.rows)
            .find(|row| &row.id == id)
    }
    pub(super) fn child_at(&self, id: &Identity) -> Option<usize> {
        let mut at = 0_usize;
        for section in &self.sections {
            if section.title.is_some() {
                at = at.saturating_add(1);
            }
            for row in &section.rows {
                if &row.id == id {
                    return Some(at);
                }
                at = at.saturating_add(1);
            }
        }
        None
    }
}

pub(super) fn project(shell: &Shell) -> Snapshot {
    let all = shell.palette_all();
    let mut registry = shell.areas.palette.sources.clone();
    let root = root(shell, &all);
    let Ok(source) = ROOT_SOURCE.parse() else {
        return Snapshot {
            sections: Vec::new(),
        };
    };
    if registry.replace(source, root).is_err() {
        return Snapshot {
            sections: Vec::new(),
        };
    }
    let query = &shell.areas.palette.query;
    if let Some(location) = &shell.areas.palette.page {
        let rows = choice_rows_search(&registry, location, query)
            .into_iter()
            .filter_map(|row| match row {
                ChoiceRow::Page { title, location } => Some(Row {
                    id: Identity::Page(location.clone()),
                    title: title.to_owned(),
                    subtitle: None,
                    glyph: None,
                    shortcut: None,
                    selected: false,
                    activate: Activate::Page(location),
                }),
                ChoiceRow::Result { id, entry } => {
                    contribution(&registry, id, entry, shell.areas.palette.keyboard_available)
                }
            })
            .collect();
        return Snapshot {
            sections: vec![Section { title: None, rows }],
        };
    }
    let mut sections: Vec<_> = contribution_sections(&registry, query)
        .into_iter()
        .map(|section| Section {
            title: section.title.map(str::to_owned),
            rows: section
                .rows
                .into_iter()
                .filter_map(|row| {
                    contribution(
                        &registry,
                        row.id,
                        row.entry,
                        shell.areas.palette.keyboard_available,
                    )
                })
                .collect(),
        })
        .collect();
    for (group, entries) in palette_groups(&all, query) {
        if group != PaletteGroup::Sessions {
            continue;
        }
        sections.push(Section {
            title: Some(shell.t.lang.palette().group(group).to_owned()),
            rows: entries
                .into_iter()
                .map(|entry| Row {
                    id: Identity::Legacy(entry.id.clone()),
                    title: if matches!(entry.target, PaletteTarget::NewSession(_)) {
                        vessel_app_view::shortcuts::action_label(Action::NewSession, shell.words)
                            .to_owned()
                    } else {
                        entry.label.clone()
                    },
                    subtitle: entry.detail.clone(),
                    glyph: Some(super::rows::glyph_of(&entry.target)),
                    shortcut: None,
                    selected: false,
                    activate: Activate::Legacy(entry.target.clone()),
                })
                .collect(),
        });
    }
    Snapshot { sections }
}

fn contribution(
    registry: &vessel_app_view::palette::registration::Registry<Command>,
    id: ResultId,
    entry: &Contribution<Command>,
    keyboard_available: bool,
) -> Option<Row> {
    if entry.target.shortcut == Some(Action::Help) && !keyboard_available {
        return None;
    }
    let (title, subtitle, selected) = match &entry.presentation {
        Presentation::Action { title, subtitle } => (
            title.clone(),
            subtitle.as_ref().map(|text| Subtitle::Plain(text.clone())),
            false,
        ),
        Presentation::Choice { path, selection } => (
            path.segments().collect::<Vec<_>>().join(" › "),
            None,
            *selection == Selection::Selected,
        ),
    };
    let activate = if id.source.as_str() == ROOT_SOURCE {
        Activate::Root(entry.target.action.clone())
    } else {
        Activate::Registered(id.clone(), registry.owner_of(&id)?)
    };
    Some(Row {
        id: Identity::Registered(id),
        title,
        subtitle,
        glyph: entry.target.glyph,
        shortcut: entry.target.shortcut,
        selected,
        activate,
    })
}

fn root(shell: &Shell, all: &[super::Entry]) -> Vec<Contribution<Command>> {
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
            _ => Presentation::Action {
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
