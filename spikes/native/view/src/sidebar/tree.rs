//! The sidebar grouped by project (TRACKER 682), as Paseo's default
//! grouping (`components/sidebar-workspace-list.tsx`): every project of
//! every machine whose list is read heads its sessions, foldable; then, per
//! machine, the sessions in no listed project.
//!
//! [Decision Log]
//! - Purpose: the native sidebar's default reads as Paseo's tree.
//! - Membership: a session is under its project's header when the node
//!   named the project (`CardDetail::runs_in`, protocol 10: its root, a
//!   folder of it, or one of its worktrees) and its machine, reachable, has
//!   that project in its list; otherwise it is in its machine's trailing
//!   group (the coordinator's ruling: one group per machine, headed by its
//!   name and state, so an unreachable machine stays in sight). An
//!   unreachable machine's projects are not listed, as the projects section
//!   lists only reachable machines': a press on them could not start
//!   anything; its sessions wait under its name.
//! - Under an open project (`sidebar-group-visibility.ts`): its first
//!   [`PROJECT_SESSIONS`] sessions and every one not done; the open one
//!   first when it would be hidden; "Show more" when any is hidden, then
//!   every one ("Show less").
//! - Order: the projects as last dropped (TRACKER 811, `ProjectOrder`),
//!   then the ones never placed by name as people read them; sessions as
//!   every grouping lists them (`listed`: pinned first, the chosen time).
//! - A trailing group lists its machine's first [`SIDEBAR_SESSIONS`], as
//!   a machine group does; it is drawn when it has sessions or its machine
//!   is not reachable.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::iter::Peekable;
use std::str::Chars;

use vessel_proto::workspace::ProjectInfo;
use vessel_proto::{ProjectId, SessionId};

use super::{SIDEBAR_SESSIONS, SidebarMachine, listed};
use crate::attention::Bucket;
use crate::board::{Card, Fleet, Reach};
use crate::prefs::{FoldedProjects, ProjectOrder, SidebarView};
use crate::workspace::projects::ProjectsView;

/// How many of a project's sessions show before "Show more" (Paseo's 7).
pub const PROJECT_SESSIONS: usize = 7;

/// What the tree is drawn with beyond the fleet.
#[derive(Debug, Clone, Copy)]
pub struct TreeState<'s> {
    /// Each machine's projects, as last read.
    pub lists: &'s HashMap<String, ProjectsView>,
    /// The folded projects.
    pub folded: &'s FoldedProjects,
    /// The projects' order (a drag's own while one is under way).
    pub order: &'s ProjectOrder,
    /// The projects showing every session ("Show more" pressed).
    pub unfolded_more: &'s HashSet<(String, ProjectId)>,
    /// The session open now, if any.
    pub open: Option<(&'s str, SessionId)>,
}

/// One project's header and its sessions.
#[derive(Debug, Clone)]
pub struct SidebarProject<'a> {
    /// Its machine.
    pub machine: &'a str,
    /// The project.
    pub project: &'a ProjectInfo,
    /// Whether it is folded (no session drawn).
    pub folded: bool,
    /// The open session when the limit would hide it, drawn first.
    pub leading: Option<&'a Card>,
    /// The sessions drawn (after `leading`).
    pub sessions: Vec<&'a Card>,
    /// Whether "Show more" / "Show less" is offered.
    pub can_toggle: bool,
    /// Whether every session shows ("Show less" offered).
    pub expanded: bool,
    /// How many sessions it has, drawn or not.
    pub total: usize,
}

/// The sidebar grouped by project.
#[derive(Debug, Clone)]
pub struct SidebarTree<'a> {
    /// The projects, placed ones first, then by name.
    pub projects: Vec<SidebarProject<'a>>,
    /// Each machine's sessions in no listed project, in the book's order.
    pub machines: Vec<SidebarMachine<'a>>,
    /// Whether more than one machine has projects (a header then names its
    /// machine).
    pub several_machines: bool,
}

impl<'a> SidebarTree<'a> {
    /// The sessions as drawn, top to bottom: what "session 1 to 9" counts
    /// and "next" and "previous" walk.
    #[must_use]
    pub fn order(&self) -> Vec<&'a Card> {
        let projects = self
            .projects
            .iter()
            .filter(|project| !project.folded)
            .flat_map(|project| project.leading.iter().chain(&project.sessions).copied());
        let loose = self
            .machines
            .iter()
            .flat_map(|machine| machine.sessions.iter().copied());
        projects.chain(loose).collect()
    }
}

impl Fleet {
    /// The sidebar grouped by project, in `view`'s order.
    #[must_use]
    pub fn sidebar_tree<'a>(&'a self, view: SidebarView, state: TreeState<'a>) -> SidebarTree<'a> {
        let mut projects = Vec::new();
        let mut machines = Vec::new();
        let mut with_projects = 0_usize;
        for machine in &self.machines {
            let shown = listed(&machine.sessions, view);
            let none: &[ProjectInfo] = &[];
            let list = state
                .lists
                .get(&machine.name)
                .filter(|_| machine.reach == Reach::Online)
                .and_then(ProjectsView::list)
                .map_or(none, |list| list.projects.as_slice());
            if !list.is_empty() {
                with_projects = with_projects.saturating_add(1);
            }
            for project in list {
                let members: Vec<&Card> = shown
                    .iter()
                    .copied()
                    .filter(|card| runs_in(card) == Some(project.id))
                    .collect();
                projects.push(project_group(&machine.name, project, members, &state));
            }
            let loose: Vec<&Card> = shown
                .into_iter()
                .filter(|card| {
                    runs_in(card).is_none_or(|id| list.iter().all(|project| project.id != id))
                })
                .collect();
            if !loose.is_empty() || machine.reach != Reach::Online {
                machines.push(SidebarMachine {
                    machine,
                    connected: machine.reach == Reach::Online,
                    live: loose.iter().filter(|card| card.live).count(),
                    more: loose.len().saturating_sub(SIDEBAR_SESSIONS),
                    sessions: loose.into_iter().take(SIDEBAR_SESSIONS).collect(),
                });
            }
        }
        let place = |project: &SidebarProject<'_>| {
            state.order.place(project.machine, project.project.id)
        };
        projects.sort_by(|one, two| match (place(one), place(two)) {
            (Some(first), Some(second)) => first.cmp(&second),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => by_name(one.project.name.as_str(), two.project.name.as_str())
                .then_with(|| one.machine.cmp(two.machine))
                .then_with(|| one.project.id.cmp(&two.project.id)),
        });
        SidebarTree {
            projects,
            machines,
            several_machines: with_projects > 1,
        }
    }
}

fn runs_in(card: &Card) -> Option<ProjectId> {
    card.detail.runs_in.map(crate::board::RunsIn::project)
}

/// A project's header and the sessions it shows (Paseo's
/// `selectSidebarGroupVisibility`, policy "workspaces").
fn project_group<'a>(
    machine: &'a str,
    project: &'a ProjectInfo,
    members: Vec<&'a Card>,
    state: &TreeState<'a>,
) -> SidebarProject<'a> {
    let key = (machine.to_owned(), project.id);
    let expanded = state.unfolded_more.contains(&key);
    let total = members.len();
    let mut baseline = Vec::new();
    let mut leading = None;
    let mut can_toggle = false;
    for (index, card) in members.iter().copied().enumerate() {
        if index < PROJECT_SESSIONS || card.bucket() != Bucket::Done {
            baseline.push(card);
        } else if leading.is_none() && is_open(card, state.open) {
            leading = Some(card);
        } else {
            can_toggle = true;
        }
    }
    let (leading, sessions) = if expanded || !can_toggle {
        (None, members)
    } else {
        (leading, baseline)
    };
    SidebarProject {
        machine,
        project,
        folded: state.folded.contains(machine, project.id),
        leading,
        sessions,
        can_toggle,
        expanded: expanded && can_toggle,
        total,
    }
}

fn is_open(card: &Card, open: Option<(&str, SessionId)>) -> bool {
    open.is_some_and(|(machine, session)| card.machine == machine && card.session == session)
}

/// Names as people read them (Paseo's `localeCompare` with `numeric`):
/// letters without regard to case, a run of digits by its value.
#[must_use]
pub fn by_name(one: &str, two: &str) -> Ordering {
    let (mut left, mut right) = (one.chars().peekable(), two.chars().peekable());
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) if a.is_ascii_digit() && b.is_ascii_digit() => {
                let (a, b) = (digits(&mut left), digits(&mut right));
                let (a, b) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
                let order = a.len().cmp(&b.len()).then_with(|| a.cmp(b));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(a), Some(b)) => {
                let order = a.to_lowercase().cmp(b.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                let _a = left.next();
                let _b = right.next();
            }
        }
    }
}

fn digits(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut run = String::new();
    while let Some(digit) = chars.next_if(char::is_ascii_digit) {
        run.push(digit);
    }
    run
}
