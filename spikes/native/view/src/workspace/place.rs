//! Where a new session runs (the new-session composer's tray): a typed
//! folder, or a project's folder as it is, a new worktree on a new branch
//! from a base, or a worktree on an existing branch (agent-orchestrator's
//! spawn screen, GOAT's environment selector, Paseo's "where"). The page's
//! `data/workspace/place.ts`, ported with its cases.
//!
//! [Decision Log]
//! - Purpose: the node has projects and a worktree per session (protocol
//!   10); the composer must start a session in one without a typed path.
//! - Constraint: a session with a `place` has no `cwd` (the node fills
//!   it); a worktree must exist, and its setup must have ended, before
//!   `session.create` names it (the node answers `conflict` while setup
//!   runs). A machine older than protocol 10 knows no projects.
//! - Alternatives: a place as a free-form path with a project guessed from
//!   it (a guess cannot pick a branch).
//! - Choice: a closed set of places here, the spec's part made from it,
//!   and whether it needs a worktree first; the steps of a start are
//!   [`super::start`]'s.
//! - Cost: the typed folder stays for machines without projects and for a
//!   folder no project holds, so there are two ways to name a folder.
//!
//! The seam with the composer (the desktop lead's): the composer holds a
//! [`Place`], draws this module's tray, asks [`place_problem`] before Send,
//! and hands a place that needs a worktree to [`super::start`].

use std::path::PathBuf;

use vessel_proto::workspace::{
    BranchName, ProjectInfo, ProjectState, RefName, SessionPlace, SetupChoice, WorktreeBase,
    WorktreeBranch,
};
use vessel_proto::{ProjectId, SessionSpec};

use super::branch_name::{BranchNameProblem, branch_name_problem};

/// How a session uses a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceMode {
    /// The project's folder as it is ("Local").
    Folder,
    /// A new worktree on a new branch.
    Worktree,
    /// A worktree on a branch that exists.
    Branch,
}

/// A project chosen in the tray.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPlace {
    /// Which project.
    pub project: ProjectId,
    /// How the session uses it.
    pub mode: PlaceMode,
    /// A new worktree's base (`None`: the project's default branch).
    pub base: Option<RefName>,
    /// A new worktree's branch name (empty: the machine names it), or the
    /// existing branch to run on, as typed.
    pub branch: String,
    /// Whether the worktree's setup runs.
    pub setup: SetupChoice,
}

/// What the tray holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// No project: the folder typed (empty: the machine's home folder).
    Path {
        /// As typed.
        folder: String,
    },
    /// A project.
    Project(ProjectPlace),
}

impl Default for Place {
    /// The tray before anything is chosen.
    fn default() -> Self {
        Self::Path {
            folder: String::new(),
        }
    }
}

impl ProjectPlace {
    /// A project chosen, in its folder as it is, as agent-orchestrator's
    /// "Local" place: the least surprising start.
    #[must_use]
    pub const fn new(project: ProjectId) -> Self {
        Self {
            project,
            mode: PlaceMode::Folder,
            base: None,
            branch: String::new(),
            setup: SetupChoice::Run,
        }
    }
}

/// The spec's part for a place that runs without a new worktree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpecPart {
    /// The folder, for a typed one.
    pub cwd: Option<PathBuf>,
    /// The project, for a project's folder.
    pub place: Option<SessionPlace>,
}

impl SpecPart {
    /// `spec` with this part in it.
    #[must_use]
    pub fn applied(self, mut spec: SessionSpec) -> SessionSpec {
        spec.cwd = self.cwd;
        spec.place = self.place.map(Box::new);
        spec
    }
}

/// The spec's part for `place` when it runs without a new worktree;
/// `None` when it needs one first.
#[must_use]
pub fn place_spec(place: &Place) -> Option<SpecPart> {
    match place {
        Place::Path { folder } => {
            let folder = folder.trim();
            Some(SpecPart {
                cwd: (!folder.is_empty()).then(|| PathBuf::from(folder)),
                place: None,
            })
        }
        Place::Project(chosen) => match chosen.mode {
            PlaceMode::Folder => Some(SpecPart {
                cwd: None,
                place: Some(SessionPlace::Project {
                    project: chosen.project,
                    dir: None,
                }),
            }),
            PlaceMode::Worktree | PlaceMode::Branch => None,
        },
    }
}

/// The branch a worktree for `place` is made on; `Ok(None)` when the place
/// needs no worktree, an error when the name typed cannot be one (which
/// [`place_problem`] says first).
///
/// # Errors
/// The branch typed is not a branch name, or none was typed for an
/// existing branch.
pub fn worktree_branch(place: &Place) -> Result<Option<WorktreeBranch>, PlaceProblem> {
    let Place::Project(chosen) = place else {
        return Ok(None);
    };
    let name = chosen.branch.trim();
    match chosen.mode {
        PlaceMode::Folder => Ok(None),
        PlaceMode::Worktree => Ok(Some(WorktreeBranch::New {
            name: if name.is_empty() {
                None
            } else {
                Some(parse_branch(name)?)
            },
            base: chosen.base.clone().map(|name| WorktreeBase::Ref { name }),
        })),
        PlaceMode::Branch => {
            if name.is_empty() {
                return Err(PlaceProblem::ChooseBranch);
            }
            Ok(Some(WorktreeBranch::Existing {
                name: parse_branch(name)?,
            }))
        }
    }
}

fn parse_branch(name: &str) -> Result<BranchName, PlaceProblem> {
    if let Some(problem) = branch_name_problem(name) {
        return Err(PlaceProblem::BranchName(problem));
    }
    // The node's parser and git's rules above agree; a name git takes that
    // the wire's type still refuses reads as "not a branch name".
    name.parse()
        .map_err(|_refused| PlaceProblem::BranchName(BranchNameProblem::Reserved("")))
}

/// Why Send cannot start in a place yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceProblem {
    /// The project is not in the machine's list.
    NotOnMachine,
    /// Its clone runs.
    StillCloning,
    /// Its clone failed.
    CloneFailed,
    /// Its folder is gone.
    FolderGone,
    /// A state a newer node knows.
    UnknownState,
    /// A worktree was asked of a folder that is not a repository.
    NotGit,
    /// An existing branch, with none chosen.
    ChooseBranch,
    /// The branch typed is not a name git takes.
    BranchName(BranchNameProblem),
}

/// Why Send cannot start in `place` yet (`None`: it can). `project` is the
/// chosen project as last listed, `None` when it is not there.
#[must_use]
pub fn place_problem(place: &Place, project: Option<&ProjectInfo>) -> Option<PlaceProblem> {
    let Place::Project(chosen) = place else {
        return None;
    };
    let Some(project) = project else {
        return Some(PlaceProblem::NotOnMachine);
    };
    match project.state {
        ProjectState::Ready => {}
        ProjectState::Cloning { .. } => return Some(PlaceProblem::StillCloning),
        ProjectState::CloneFailed { .. } => return Some(PlaceProblem::CloneFailed),
        ProjectState::Missing => return Some(PlaceProblem::FolderGone),
        ProjectState::Unknown => return Some(PlaceProblem::UnknownState),
    }
    if chosen.mode == PlaceMode::Folder {
        return None;
    }
    if !project.git {
        return Some(PlaceProblem::NotGit);
    }
    let name = chosen.branch.trim();
    if chosen.mode == PlaceMode::Branch && name.is_empty() {
        return Some(PlaceProblem::ChooseBranch);
    }
    if name.is_empty() {
        None
    } else {
        branch_name_problem(name).map(PlaceProblem::BranchName)
    }
}
