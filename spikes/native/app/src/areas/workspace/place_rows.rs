//! The place lists' rows and the tray's pills' words (`place`'s state,
//! read): Paseo's project rows (`ProjectOption`: the project's mark, its
//! name, its folder beside it), its isolation rows (`IsolationOption`) and
//! its ref rows (`RefOption`: a branch glyph, the name, the distance from
//! upstream at the right as `+2 −1`).
//!
//! Differences from Paseo, named: a remote-tracking branch is listed by
//! git's own name (`origin/main`) where Paseo writes `main` and marks the
//! local one `main (local)`; pull requests join the list with the forge
//! (track F's item 4).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::SharedString;
use vessel_app_kit::{ComboRow, Leading, initial_of};
use vessel_app_view::workspace::place::PlaceMode;
use vessel_proto::workspace::ProjectState;

use super::place::{Branches, PlacePick, Rows};
use crate::shell::Shell;

/// What the tray's pills show for a chosen project.
pub(crate) struct PlacePills {
    /// The project's name and its mark.
    pub(crate) name: SharedString,
    pub(crate) initial: SharedString,
    /// Isolation's value and glyph, for a repository.
    pub(crate) isolation: Option<(&'static str, Glyph)>,
    /// Start from's value, for a new worktree.
    pub(crate) base: Option<SharedString>,
}

fn matches(text: &str, query: &str) -> bool {
    query.is_empty() || text.to_lowercase().contains(query)
}

/// A branch's distance from its upstream, as Paseo's ref picker writes it
/// (`+2 −1`; nothing when level).
fn divergence(ahead: u32, behind: u32) -> Option<SharedString> {
    let parts: Vec<String> = [
        (ahead > 0).then(|| format!("+{ahead}")),
        (behind > 0).then(|| format!("\u{2212}{behind}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(" ").into())
}

impl Shell {
    /// The draft machine's projects, as rows of the folder pill's list
    /// (Paseo's project rows): those that match `query` (lower case) by
    /// name or folder, the chosen one checked, one not ready greyed with
    /// its state.
    pub(crate) fn project_rows(&self, query: &str) -> Rows {
        let words = self.words();
        let Some(listed) = self
            .areas
            .draft
            .machine
            .as_deref()
            .and_then(|machine| self.areas.workspace.projects.get(machine))
            .and_then(|view| view.list())
        else {
            return Vec::new();
        };
        let chosen = self.chosen_place().map(|place| place.project);
        let mut rows = Vec::new();
        for project in &listed.projects {
            let name = project.name.to_string();
            let folder = project.root.display().to_string();
            if !matches(&name, query) && !matches(&folder, query) {
                continue;
            }
            let mut row = ComboRow::new(name.clone());
            row.leading = Some(Leading::Initial(initial_of(&name)));
            row.detail = Some(folder.into());
            row.checked = chosen == Some(project.id);
            if !matches!(project.state, ProjectState::Ready) {
                row.disabled = true;
                row.trailing = words.project_state(&project.state).map(Into::into);
            }
            if rows.is_empty() {
                row.heading = Some(words.labels().projects.into());
            }
            rows.push((row, PlacePick::Project(project.id)));
        }
        rows
    }

    /// The list's last row: Paseo's "Add project" footer, for a machine
    /// that has projects (protocol 10 and later, its list read).
    pub(crate) fn add_project_row(&self) -> Option<(ComboRow, PlacePick)> {
        let _listed = self
            .areas
            .draft
            .machine
            .as_deref()
            .and_then(|machine| self.areas.workspace.projects.get(machine))?
            .list()?;
        let mut row = ComboRow::new(self.words().labels().add_project);
        row.leading = Some(Leading::Glyph(Glyph::Plus));
        Some((row, PlacePick::AddProject))
    }

    /// Isolation's rows: Local, and New worktree (Paseo leaves it out for
    /// a folder that is not a repository; so does the tray's pill).
    pub(crate) fn isolation_rows(&self) -> Rows {
        let labels = self.words().labels();
        let worktree = self
            .chosen_place()
            .is_some_and(|place| place.mode != PlaceMode::Folder);
        let mut local = ComboRow::new(labels.local);
        local.leading = Some(Leading::Glyph(Glyph::Folder));
        local.checked = !worktree;
        let mut new = ComboRow::new(labels.new_worktree);
        new.leading = Some(Leading::Glyph(Glyph::Branch));
        new.checked = worktree;
        vec![
            (local, PlacePick::Isolation(PlaceMode::Folder)),
            (new, PlacePick::Isolation(PlaceMode::Worktree)),
        ]
    }

    /// Start from's rows (the project's branches matching `query`, local
    /// ones first, each with its distance from upstream) and what the list
    /// says when there are none.
    pub(crate) fn base_rows(&self, query: &str) -> (Rows, String) {
        let words = self.words();
        let labels = words.labels();
        let (Some(machine), Some(place)) = (self.areas.draft.machine.clone(), self.chosen_place())
        else {
            return (Vec::new(), labels.no_matching_refs.to_owned());
        };
        let list = match self
            .areas
            .workspace
            .place
            .branches
            .get(&(machine, place.project))
        {
            None | Some(Branches::Loading) => {
                return (Vec::new(), labels.reading_branches.to_owned());
            }
            Some(Branches::Failed(problem)) => return (Vec::new(), words.problem(problem)),
            Some(Branches::Ready(list)) => list,
        };
        let mut rows = Vec::new();
        for branch in &list.branches {
            let name = branch.name.as_str();
            if !matches(name, query) {
                continue;
            }
            let mut row = ComboRow::new(name.to_owned());
            row.leading = Some(Leading::Glyph(Glyph::Branch));
            row.checked = place
                .base
                .as_ref()
                .map_or(branch.default, |base| base == &branch.name);
            row.trailing = branch
                .upstream
                .as_ref()
                .and_then(|distance| divergence(distance.ahead, distance.behind));
            rows.push((row, PlacePick::Base(Some(branch.name.clone()))));
        }
        (rows, labels.no_matching_refs.to_owned())
    }

    /// The pills' words for the chosen project; `None` with no project (the
    /// folder pill shows the folder).
    pub(crate) fn place_pills(&self) -> Option<PlacePills> {
        let place = self.chosen_place()?;
        let project = self.chosen_project();
        let labels = self.words().labels();
        let name = project.map_or_else(String::new, |project| project.name.to_string());
        let git = project.is_some_and(|project| project.git);
        let worktree = place.mode != PlaceMode::Folder;
        Some(PlacePills {
            initial: initial_of(&name),
            name: name.into(),
            isolation: git.then_some(if worktree {
                (labels.new_worktree, Glyph::Branch)
            } else {
                (labels.local, Glyph::Folder)
            }),
            base: (git && worktree).then(|| {
                place
                    .base
                    .as_ref()
                    .or_else(|| project.and_then(|project| project.default_branch.as_ref()))
                    .map_or_else(
                        || labels.default_ref.into(),
                        |base| base.as_str().to_owned().into(),
                    )
            }),
        })
    }
}
