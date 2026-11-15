//! Each machine's projects as last read, shared by the sidebar's group,
//! Settings > Projects, the composer's tray and a session's panel; a clone
//! is followed until it ends (the page's `data/workspace/project-store.ts`:
//! the decisions here, the reads and the following in the link).
//!
//! [Decision Log]
//! - Purpose: four screens show the same list; each asking `project.list`
//!   on its own would read it four times and show four states.
//! - Constraint: the node pushes nothing when a project changes; a clone's
//!   progress is followed with `project.get { changed_from }` (a wait of at
//!   most 30 s on the node, answered when the revision differs). A machine
//!   older than protocol 10 has no projects at all.
//! - Alternatives: the fleet's own list (read every two seconds for
//!   sessions): projects change rarely, and that list is the lead's.
//! - Choice: a view of its own per authenticated peer, with route aliases used
//!   only after `NodeId` validation. Read tickets reject superseded completions;
//!   reconnect requires hydration, and clone revisions cannot move backwards.
//!   The link follows each cloning project on that exact node until it ends.
//! - Cost: a project added from the CLI shows when a screen reads again.

/// Host-bound mutation correlation shared by registration and removal screens.
pub mod actions;
/// Pure Add Project page stack and target-scoped in-flow drafts; screen wiring is separate.
pub mod navigation;
/// Authenticated peer and request-order correlation for cached project reads.
pub mod reads;

use std::path::Path;

use vessel_proto::ProjectId;
use vessel_proto::workspace::{ProjectInfo, ProjectList, ProjectState};

use super::problem::Problem;

/// What is known of a machine's projects.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProjectsView {
    /// Being read.
    #[default]
    Loading,
    /// Read.
    Ready(ProjectList),
    /// The read failed.
    Failed(Problem),
    /// The machine speaks a protocol older than projects.
    Older,
}

impl ProjectsView {
    /// The view after a `project.list` answered.
    #[must_use]
    pub fn listed(answer: Result<ProjectList, Problem>) -> Self {
        match answer {
            Ok(list) => Self::Ready(list),
            Err(Problem::Older) => Self::Older,
            Err(problem) => Self::Failed(problem),
        }
    }

    /// Merge a fresh list without regressing a newer clone progress revision.
    pub fn listed_after(&mut self, mut answer: Result<ProjectList, Problem>) {
        if let (Self::Ready(previous), Ok(next)) = (&*self, &mut answer) {
            for project in &mut next.projects {
                if let Some(known) = previous
                    .projects
                    .iter()
                    .find(|known| known.id == project.id)
                    && known.revision > project.revision
                {
                    *project = known.clone();
                }
            }
        }
        *self = Self::listed(answer);
    }

    /// The list, when it was read.
    #[must_use]
    pub const fn list(&self) -> Option<&ProjectList> {
        match self {
            Self::Ready(list) => Some(list),
            Self::Loading | Self::Failed(_) | Self::Older => None,
        }
    }

    /// A project by id.
    #[must_use]
    pub fn project(&self, id: &ProjectId) -> Option<&ProjectInfo> {
        self.list()?
            .projects
            .iter()
            .find(|project| &project.id == id)
    }

    /// `project` as it is now, in the list (the answer to a follower's
    /// read); nothing when the list was not read or does not hold it.
    pub fn replace(&mut self, project: ProjectInfo) {
        if let Self::Ready(list) = self
            && let Some(known) = list
                .projects
                .iter_mut()
                .find(|known| known.id == project.id)
            && project.revision >= known.revision
        {
            *known = project;
        }
    }

    /// A project the node just answered with (added, cloned, started) into
    /// the list at once: in its place when known, else last.
    pub fn took(&mut self, project: ProjectInfo) {
        if let Self::Ready(list) = self {
            match list
                .projects
                .iter_mut()
                .find(|known| known.id == project.id)
            {
                Some(known) => *known = project,
                None => list.projects.push(project),
            }
        }
    }

    /// A forgotten project out of the list.
    pub fn forgot(&mut self, id: &ProjectId) {
        if let Self::Ready(list) = self {
            list.projects.retain(|project| &project.id != id);
        }
    }

    /// The projects being cloned, which a follower keeps current.
    pub fn cloning(&self) -> impl Iterator<Item = &ProjectInfo> {
        self.list()
            .into_iter()
            .flat_map(|list| list.projects.iter())
            .filter(|project| is_cloning(project))
    }
}

/// Whether `project`'s clone runs.
#[must_use]
pub const fn is_cloning(project: &ProjectInfo) -> bool {
    matches!(project.state, ProjectState::Cloning { .. })
}

/// The project of `projects` whose folder holds `cwd`, the deepest (one
/// project may be inside another), for a session started in a typed
/// folder. Folders are compared as text: a link to a project's folder is
/// not followed.
#[must_use]
pub fn project_among<'a>(
    projects: &'a [ProjectInfo],
    cwd: Option<&Path>,
) -> Option<&'a ProjectInfo> {
    let cwd = cwd?.to_string_lossy();
    let folder = cwd.trim_end_matches(['/', '\\']);
    let mut best: Option<(&ProjectInfo, usize)> = None;
    for project in projects {
        let root_text = project.root.to_string_lossy();
        let root = root_text.trim_end_matches(['/', '\\']);
        let inside = folder == root
            || folder
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('\\'));
        if inside && best.is_none_or(|(_, length)| root.len() > length) {
            best = Some((project, root.len()));
        }
    }
    best.map(|(project, _)| project)
}

/// The repository name a clone URL ends with (`org/shop.git` gives
/// `shop`), for the clone form's help line (the page's `repositoryName`).
#[must_use]
pub fn repository_name(url: &str) -> Option<&str> {
    let trimmed = url.trim().trim_end_matches('/');
    let last = trimmed.rsplit(['/', ':']).next().unwrap_or_default();
    let name = last.strip_suffix(".git").unwrap_or(last);
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use vessel_proto::workspace::{CloneStage, GitSupport, GitVersion};

    use super::*;

    const P1: &str = "00000000-0000-4000-8000-000000000001";
    const P2: &str = "00000000-0000-4000-8000-000000000002";

    fn project(id: &str, root: &str) -> ProjectInfo {
        ProjectInfo {
            id: id.parse().unwrap(),
            name: "p".parse().unwrap(),
            root: PathBuf::from(root),
            git: true,
            default_branch: None,
            state: ProjectState::Ready,
            created_at: Utc::now(),
            revision: 1,
        }
    }

    fn list(projects: Vec<ProjectInfo>) -> ProjectList {
        ProjectList {
            git: GitSupport::Found {
                version: GitVersion::MINIMUM,
            },
            projects,
            more: false,
        }
    }

    #[test]
    fn finds_the_deepest_project_holding_a_folder() {
        let outer = project(P1, "/work");
        let inner = project(P2, "/work/vessel/");
        let projects = [outer, inner];
        let found = project_among(&projects, Some(Path::new("/work/vessel/web")));
        assert_eq!(
            found.map(|project| project.id.to_string()),
            Some(P2.to_owned())
        );
        let other = project_among(&projects, Some(Path::new("/workshop")));
        assert!(other.is_none());
        assert!(project_among(&projects, None).is_none());
    }

    #[test]
    fn takes_a_project_in_its_place_or_last_and_follows_the_cloning() {
        let mut view = ProjectsView::listed(Ok(list(vec![project(P1, "/a")])));
        let mut cloning = project(P2, "/b");
        cloning.state = ProjectState::Cloning {
            stage: CloneStage::Receiving,
            percent: Some(3),
        };
        view.took(cloning.clone());
        assert_eq!(view.cloning().count(), 1);
        cloning.state = ProjectState::Ready;
        view.replace(cloning);
        assert_eq!(view.cloning().count(), 0);
        assert_eq!(view.list().map(|list| list.projects.len()), Some(2));
        view.forgot(&P1.parse().unwrap());
        assert_eq!(view.list().map(|list| list.projects.len()), Some(1));
    }

    #[test]
    fn reordered_list_and_follow_responses_cannot_regress_clone_completion() {
        let mut ready = project(P1, "/fixture");
        ready.revision = 3;
        let mut view = ProjectsView::Ready(list(vec![ready.clone()]));
        let mut stale = ready.clone();
        stale.revision = 1;
        stale.state = ProjectState::Cloning {
            stage: CloneStage::Receiving,
            percent: Some(50),
        };
        view.replace(stale.clone());
        assert_eq!(view.project(&ready.id), Some(&ready));
        view.listed_after(Ok(list(vec![stale])));
        assert_eq!(view.project(&ready.id), Some(&ready));
        assert_eq!(view.cloning().count(), 0);
    }

    #[test]
    fn names_the_repository_a_clone_url_ends_with() {
        assert_eq!(
            repository_name("https://github.com/org/shop.git"),
            Some("shop")
        );
        assert_eq!(
            repository_name("git@github.com:org/shop.git/ "),
            Some("shop")
        );
        assert_eq!(repository_name("https://host/org/shop"), Some("shop"));
        assert_eq!(repository_name("  "), None);
        assert_eq!(repository_name("https://host/.git"), None);
    }

    #[test]
    fn reads_an_older_machine_as_one_without_projects() {
        assert_eq!(
            ProjectsView::listed(Err(Problem::Older)),
            ProjectsView::Older
        );
    }
}
