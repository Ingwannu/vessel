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
    Contribution, Owner, Presentation, ResultId, Selection,
};
use vessel_app_view::palette::{PaletteGroup, Subtitle, contribution_sections, palette_groups};
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
    let root = super::root::build(shell, &all);
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
                    title: entry.label.clone(),
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
